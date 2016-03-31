use rotor_http::server::Fsm as HttpFsm;
use rotor_http::server::{Head, Server, Response, RecvMode, HttpError};
use rotor::{Scope, Time};
use std::time::Duration;
use url::{self, ParseError, form_urlencoded};
use std::str;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::rc::Rc;
use super::models::Event;
use super::context::Context;
use mime::{Mime, TopLevel, SubLevel, Value, Attr};
use serde_json;
use multipart::server::{Multipart, MultipartData};
use std::io::Read;

const KEEPALIVE_TIMEOUT_SECS: u64 = 30;

enum Error {
    BadRequest,
    Forbidden,
    MethodNotAllowed,

    InternalServerError,
}

impl HttpError for Error {
    fn http_status(&self) -> (u16, &'static str) {
        match *self {
            Error::BadRequest => (400, "Bad Request"),
            Error::Forbidden => (403, "Forbidden"),
            Error::MethodNotAllowed => (405, "Method Not Allowed"),
            Error::InternalServerError => (500, "Internal Server Error"),
        }
    }
}

fn parse_path(path: &str) -> Result<String, ParseError> {
    let (path, _, _) = try!(url::parse_path(path));
    let mut endpoint = String::new();
    for segment in path {
        endpoint.push('/');
        endpoint.push_str(&segment);
    }
    Ok(endpoint)
}

macro_rules! try_or_bad_request {
    ($e:expr, $response:expr, $seed:expr, $scope:expr) => (match log_result!($e) {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(_) => {
            Machine::emit_error_page(&Error::BadRequest, $response, &$seed, $scope);
            return None
        },
    })
}

macro_rules! try_or_500 {
    ($e:expr, $response:expr, $seed:expr, $scope:expr) => (match log_result!($e) {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(_) => {
            Machine::emit_error_page(&Error::InternalServerError, $response, &$seed, $scope);
            return None
        },
    })
}

#[derive(Clone)]
pub struct Seed;

pub type Fsm<L> = HttpFsm<Machine, L>;

#[derive(Debug)]
enum ContentType {
    /// application/x-www-form-urlencoded
    Urlencoded,
    /// multipart/form-data; boundary=...
    FormData(String),
    /// application/json
    Json,
}

#[derive(Debug)]
enum Request {
    Head,
    Get { last_event_id: Option<i64>, endpoint: String },
    Post { content_type: ContentType, endpoint: String, body: Vec<u8> },
}

#[derive(Debug)]
enum State {
    Body(Request),
    Response(Request),
}

#[derive(Debug)]
pub struct Machine {
    channel: Receiver<Rc<Event>>,
    state: State,
}

impl Server for Machine {
    type Context = Context;
    type Seed = Seed;

    fn headers_received(seed: Self::Seed,
                        head: Head,
                        response: &mut Response,
                        scope: &mut Scope<Self::Context>)
                        -> Option<(Self, RecvMode, Time)> {
        let recv_mode = RecvMode::Progressive(4096);
        let timeout = scope.now() + Duration::from_secs(KEEPALIVE_TIMEOUT_SECS);
        match head.method {
            "HEAD" => {
                let endpoint = try_or_bad_request!(parse_path(head.path), response, seed, scope);
                info!("HEAD {}", endpoint);
                let (tx, rx) = mpsc::channel();
                let notifier = scope.notifier();
                scope.register_client(notifier, tx);
                Some((Machine {
                    channel: rx,
                    state: State::Body(Request::Head),
                }, recv_mode, timeout))
            },
            "GET" => {
                let endpoint = try_or_bad_request!(parse_path(head.path), response, seed, scope);
                let mut last_event_id = None;
                for header in head.headers {
                    let equal = header.name.chars()
                        .flat_map(char::to_lowercase)
                        .eq("last-event-id".chars());
                    if equal {
                        let value = try_or_bad_request!(str::from_utf8(header.value), response, seed, scope);
                        last_event_id = Some(try_or_bad_request!(value.parse::<i64>(), response, seed, scope));
                        break
                    }
                }
                info!("GET {}, Last-Event-Id: {:?}", endpoint, last_event_id);
                let (tx, rx) = mpsc::channel();
                let notifier = scope.notifier();
                scope.register_client(notifier, tx);
                Some((Machine {
                    channel: rx,
                    state: State::Body(Request::Get {
                        last_event_id: last_event_id,
                        endpoint: endpoint
                    }),
                }, recv_mode, timeout))
            },
            "POST" => {
                let endpoint = try_or_bad_request!(parse_path(head.path), response, seed, scope);
                info!("POST {}", endpoint);

                let mut content_type = None;
                for header in head.headers {
                    let equal = header.name.chars()
                        .flat_map(char::to_lowercase)
                        .eq("content-type".chars());
                    if equal {
                        let value = try_or_bad_request!(str::from_utf8(header.value), response, seed, scope);
                        content_type = Some(try_or_bad_request!(value.parse::<Mime>().map_err(|()| format!("Failed to parse {:?}", value)), response, seed, scope));
                        break
                    }
                }

                let content_type = match content_type {
                    Some(Mime(TopLevel::Application, SubLevel::WwwFormUrlEncoded, _)) => ContentType::Urlencoded,
                    Some(Mime(TopLevel::Multipart, SubLevel::FormData, attrs)) => {
                        let mut boundary = None;
                        for (attr, value) in attrs {
                            match (attr, value) {
                                (Attr::Boundary, Value::Ext(value)) => {
                                    boundary = Some(value);
                                    break
                                },
                                _ => (),
                            }
                        }
                        match boundary {
                            Some(boundary) => ContentType::FormData(boundary),
                            None => {
                                Machine::emit_error_page(&Error::BadRequest, response, &seed, scope);
                                return None
                            }
                        }
                    },
                    Some(Mime(TopLevel::Application, SubLevel::Json, _)) => ContentType::Json,
                    _ => {
                        Machine::emit_error_page(&Error::BadRequest, response, &seed, scope);
                        return None
                    }
                };

                let (tx, rx) = mpsc::channel();
                let notifier = scope.notifier();
                scope.register_client(notifier, tx);
                Some((Machine {
                    channel: rx,
                    state: State::Body(Request::Post {
                        endpoint: endpoint,
                        content_type: content_type,
                        body: vec![]
                    }),
                }, recv_mode, timeout))
            },
            _method => {
                Machine::emit_error_page(&Error::MethodNotAllowed, response, &seed, scope);
                None
            },
        }
    }

    fn request_received(self,
                        _data: &[u8],
                        _response: &mut Response,
                        _scope: &mut Scope<Self::Context>)
                        -> Option<Self> {
        unreachable!()
    }

    fn request_chunk(mut self,
                     chunk: &[u8],
                     response: &mut Response,
                     scope: &mut Scope<Self::Context>)
                     -> Option<Self> {
        self.state = match self.state {
            state @ State::Body(Request::Get { .. }) | state @ State::Body(Request::Head) => {
                if chunk.len() != 0 {
                    Machine::emit_error_page(&Error::BadRequest, response, &Seed, scope);
                    return None
                } else {
                    state
                }
            },
            State::Body(Request::Post { endpoint, content_type, mut body }) => {
                body.extend_from_slice(chunk);
                State::Body(Request::Post {
                    endpoint: endpoint,
                    content_type: content_type,
                    body: body,
                })
            },
            state => {
                error!("State machine in wrong state, expected Body, found {:?}", state);
                return None
            },
        };

        Some(self)
    }

    fn request_end(mut self,
                   response: &mut Response,
                   scope: &mut Scope<Self::Context>)
                   -> Option<Self> {
        self.state = match self.state {
            State::Body(req @ Request::Head) | State::Body(req @ Request::Get { .. }) => {
                response.status(200, "OK");
                response.add_chunked().unwrap();
                response.add_header("Content-Type", b"text/event-stream; charset=utf-8").unwrap();
                // If is HEAD request
                if ! response.done_headers().unwrap() {
                    response.done();
                    return None
                } else {
                    match req {
                        Request::Get { ref endpoint, last_event_id: Some(id) } => {
                            for event in scope.get_missed_events(&endpoint, id).unwrap_or(vec![]) {
                                send_event(response, &event)
                            }
                        },
                        _ => (),
                    }
                    State::Response(req)
                }
            },
            State::Body(Request::Post { endpoint, content_type, body }) => {
                let (access_key, event, data) = match content_type {
                    ContentType::Urlencoded => {
                        let body = form_urlencoded::parse(&body);
                        let mut access_key = None;
                        let mut event = None;
                        let mut data = None;

                        for (key, value) in body {
                            match &key[..] {
                                "access-key" => access_key = Some(value),
                                "event" => event = Some(value),
                                "data" => data = Some(value),
                                _ => (),
                            }
                        }

                        (access_key, event, data)
                    },
                    ContentType::FormData(boundary) => {
                        let mut body = Multipart::with_body(&body[..], boundary);
                        let mut access_key = None;
                        let mut event = None;
                        let mut data = None;

                        try_or_bad_request!(body.foreach_entry(|entry| {
                            let value = match &entry.name[..] {
                                "access-key" => &mut access_key,
                                "event" => &mut event,
                                "data" => &mut data,
                                _ => return,
                            };
                            *value = match entry.data {
                                MultipartData::Text(data) => Some(String::from(data)),
                                MultipartData::File(mut file) => {
                                    let mut data = String::new();
                                    let res = log_result!(file.read_to_string(&mut data)).ok();
                                    res.map(|_| data)
                                }
                            }
                        }), response, &Seed, scope);

                        (access_key, event, data)
                    },
                    ContentType::Json => {
                        #[derive(Deserialize)]
                        struct Data {
                            #[serde(rename="access-key")]
                            access_key: String,
                            event: String,
                            data: String,
                        }
                        let body = try_or_bad_request!(serde_json::from_slice::<Data>(&body), response, &Seed, scope);
                        (Some(body.access_key), Some(body.event), Some(body.data))
                    }
                };

                match (access_key, event, data) {
                    (Some(access_key), Some(event), Some(data)) => {
                        if try_or_500!(scope.check_key(&endpoint, &access_key), response, &Seed, scope) {
                            try_or_500!(scope.send_event(endpoint, event, data), response, &Seed, scope);
                            response.status(200, "OK");
                            response.add_chunked().unwrap();
                            response.add_header("Content-Type", b"text/plain; charset=utf-8").unwrap();
                            response.done_headers().unwrap();
                            response.write_body(b"");
                            response.done();
                        } else {
                            Machine::emit_error_page(&Error::Forbidden, response, &Seed, scope);
                        }
                    },
                    _ => {
                        Machine::emit_error_page(&Error::BadRequest, response, &Seed, scope);
                        return None
                    },
                }

                return None
            },
            state => {
                error!("State machine in wrong state, expected Body, found {:?}", state);
                return None
            },
        };

        Some(self)
    }

    fn timeout(self,
               response: &mut Response,
               scope: &mut Scope<Self::Context>)
               -> Option<(Self, Time)> {
        match self.state {
            State::Response(Request::Get { .. }) => {
                response.write_body(b": keep-alive\n\n");
                Some((self, scope.now() + Duration::from_secs(KEEPALIVE_TIMEOUT_SECS)))
            },
            state => {
                error!("State machine in wrong state, expected Response(Get), found {:?}", state);
                None
            },
        }
    }

    fn wakeup(self, response: &mut Response, _scope: &mut Scope<Self::Context>) -> Option<Self> {
        match self.state {
            State::Response(Request::Get { ref endpoint, .. }) => loop {
                match self.channel.try_recv() {
                    Ok(event) => if &event.endpoint == endpoint {
                        send_event(response, &event)
                    },
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        response.done();
                        return None
                    },
                }
            },
            state => {
                error!("State machine in wrong state, expected Response(Get), found {:?}", state);
                return None
            },
        }

        Some(self)
    }
}

fn send_event(response: &mut Response, event: &Event) {
    response.write_body(b"event:");
    response.write_body(event.event.as_bytes());
    response.write_body(b"\n");
    for chunk in event.data.split('\n') {
        response.write_body(b"data:");
        response.write_body(chunk.as_bytes());
        response.write_body(b"\n");
    }
    response.write_body(format!("id:{}\n\n", event.id).as_bytes());
}
