use rotor_http::server::Fsm as HttpFsm;
use rotor_http::server::{Head, Server, Response, RecvMode, HttpError};
use rotor::{Scope, Time};
use std::time::Duration;
use url::{self, ParseError, form_urlencoded};
use std::str;

use super::Context;

const KEEPALIVE_TIMEOUT_SECS: u64 = 30;

enum Error {
    BadRequest,
    MethodNotAllowed,
}

impl HttpError for Error {
    fn http_status(&self) -> (u16, &'static str) {
        match *self {
            Error::BadRequest => (400, "Bad Request"),
            Error::MethodNotAllowed => (405, "Method Not Allowed"),
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

#[derive(Clone)]
pub struct Seed;

pub type Fsm<L> = HttpFsm<Machine, L>;

#[derive(Debug)]
pub enum Request {
    Head,
    Get { last_event_id: Option<i64>, endpoint: String },
    Post { endpoint: String, body: Vec<u8> },
}

#[derive(Debug)]
pub enum Machine {
    Body(Request),
    Response(Request),
}

impl Server for Machine {
    type Context = Context;
    type Seed = Seed;

    fn headers_received(seed: Self::Seed,
                        head: Head,
                        response: &mut Response,
                        scope: &mut Scope<Self::Context>)
                        -> Option<(Self, RecvMode, Time)> {
        debug!("headers_received");
        let recv_mode = RecvMode::Progressive(4096);
        let timeout = scope.now() + Duration::from_secs(KEEPALIVE_TIMEOUT_SECS);
        match head.method {
            "HEAD" => {
                let endpoint = try_or_bad_request!(parse_path(head.path), response, seed, scope);
                info!("HEAD {}", endpoint);
                Some((Machine::Body(Request::Head), recv_mode, timeout))
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
                Some((Machine::Body(Request::Get {
                    last_event_id: last_event_id,
                    endpoint: endpoint
                }), recv_mode, timeout))
            },
            "POST" => {
                let endpoint = try_or_bad_request!(parse_path(head.path), response, seed, scope);
                info!("POST {}", endpoint);
                Some((Machine::Body(Request::Post {
                    endpoint: endpoint,
                    body: vec![]
                }), recv_mode, timeout))
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
        debug!("request_received: {:?}", self);
        unreachable!()
    }

    fn request_chunk(self,
                     chunk: &[u8],
                     response: &mut Response,
                     scope: &mut Scope<Self::Context>)
                     -> Option<Self> {
        debug!("request_chunk: {:?}", self);
        match self {
            state @ Machine::Body(Request::Get { .. }) | state @ Machine::Body(Request::Head) => {
                if chunk.len() != 0 {
                    Machine::emit_error_page(&Error::BadRequest, response, &Seed, scope);
                    None
                } else {
                    Some(state)
                }
            },
            Machine::Body(Request::Post { endpoint, mut body }) => {
                body.extend_from_slice(chunk);
                Some(Machine::Response(Request::Post { endpoint: endpoint, body: body }))
            },
            state => {
                error!("State machine in wrong state, expected Body, found {:?}", state);
                None
            },
        }
    }

    fn request_end(self,
                   response: &mut Response,
                   scope: &mut Scope<Self::Context>)
                   -> Option<Self> {
        debug!("request_end: {:?}", self);
        match self {
            Machine::Body(req @ Request::Head) | Machine::Body(req @ Request::Get { .. }) => {
                response.status(200, "OK");
                response.add_chunked().unwrap();
                response.add_header("Content-Type", b"text/event-stream; charset=utf-8").unwrap();
                // If is HEAD request
                if ! response.done_headers().unwrap() {
                    response.done();
                    None
                } else {
                    Some(Machine::Response(req))
                }
            },
            Machine::Body(Request::Post { endpoint, body }) => {
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

                match (access_key, event, data) {
                    (Some(access_key), Some(event), Some(data)) => unimplemented!(),
                    _ => {
                        Machine::emit_error_page(&Error::MethodNotAllowed, response, &Seed, scope);
                        None
                    },
                }
            },
            state => {
                error!("State machine in wrong state, expected Body, found {:?}", state);
                None
            },
        }
    }

    fn timeout(self,
               response: &mut Response,
               scope: &mut Scope<Self::Context>)
               -> Option<(Self, Time)> {
        debug!("timeout: {:?}", self);
        match self {
            state @ Machine::Response(Request::Get { .. }) => {
                response.write_body(b": keep-alive\n\n");
                Some((state, scope.now() + Duration::from_secs(KEEPALIVE_TIMEOUT_SECS)))
            },
            state => {
                error!("State machine in wrong state, expected Body, found {:?}", state);
                None
            },
        }
    }

    fn wakeup(self, response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<Self> {
        debug!("wakeup: {:?}", self);
        Some(self)
    }
}
