use rotor_stream::{Accept, Accepted, StreamSocket};
use rotor::{Scope, Response, Void, GenericScope, Evented, EventSet, PollOpt};
use rotor::void::unreachable;
use rotor::mio::{TryAccept};
use super::Context;
use std::any::Any;
use std::io;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

pub type Fsm<L> = Accept<Machine<<L as TryAccept>::Output>, L>;

pub type Seed = Rc<RefCell<HashMap<String, Box<FnMut(&mut Scope<Context>, Value, Option<i32>) -> Result<Value, Value>>>>>;

pub fn new<L: TryAccept + Evented + Any, S: GenericScope>(listener: L,
                                                          seed: Seed,
                                                          scope: &mut S)
                                                          -> Response<Fsm<L>, Void>
    where L::Output: StreamSocket
{
    Fsm::new(listener, seed, scope)
}

macro_rules! rotor_try {
    ($e:expr) => (match log_result!($e) {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(e) => return ::rotor::Response::error(e.into()),
    });
}

#[derive(Debug, Deserialize)]
struct Call {
    command: String,
    param: Value,
    user: Option<i32>,
}

#[derive(Serialize)]
struct Reply {
    success: bool,
    result: Value,
}

pub struct Machine<S> {
    socket: S,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    functions: Seed,
}

impl<S: StreamSocket> ::rotor::Machine for Machine<S> {
    type Context = Context;
    type Seed = Void;

    fn create(seed: Self::Seed, _scope: &mut Scope<Context>) -> Response<Self, Void> {
        unreachable(seed)
    }

    fn ready(mut self,
             events: EventSet,
             scope: &mut Scope<Context>)
             -> Response<Self, Self::Seed> {
        if events.is_hup() {
            rotor_try!(scope.deregister(&self.socket));
            return Response::done();
        }
        if events.is_readable() {
            let _ = rotor_try!(nonblock!(io::copy(&mut self.socket, &mut self.read_buffer)));

            loop {
                let bytes = {
                    let mut iter = self.read_buffer.splitn(2, |&b| b == b'\n');
                    match (iter.next(), iter.next()) {
                        (Some(msg), Some(tail)) => {
                            let call = rotor_try!(serde_json::from_slice::<Call>(msg));
                            let response = match self.functions.borrow_mut().get_mut(&call.command) {
                                Some(func) => (func)(scope, call.param, call.user),
                                None => Err(Value::String(format!("No method named {:?}", call.command))),
                            };
                            let response = match response {
                                Ok(result) => Reply {
                                    success: true,
                                    result: result,
                                },
                                Err(error) => {
                                    error!("{:?} failed: {:?}", call.command, error);
                                    Reply {
                                        success: false,
                                        result: error,
                                    }
                                },
                            };
                            rotor_try!(serde_json::to_writer(&mut self.write_buffer, &response));
                            self.write_buffer.push(b'\n');
                            rotor_try!(scope.reregister(&self.socket, EventSet::hup() | EventSet::readable() | EventSet::writable(), PollOpt::edge()));

                            self.read_buffer.len() - tail.len()
                        },
                        (Some(_), None) => break,
                        (None, _) => unreachable!(),
                    }
                };
                for _ in self.read_buffer.drain(..bytes) {
                }
            }
        }
        if events.is_writable() {
            let bytes = {
                let mut buffer = &self.write_buffer[..];
                match rotor_try!(nonblock!(io::copy(&mut buffer, &mut self.socket))) {
                    Some(bytes) => bytes as usize,
                    None => self.write_buffer.len() - buffer.len(),
                }
            };
            for _ in self.write_buffer.drain(..bytes) {
            }
            if self.write_buffer.len() == 0 {
                rotor_try!(scope.reregister(&self.socket, EventSet::hup() | EventSet::readable(), PollOpt::edge()));
            }
        }

        Response::ok(self)
    }

    fn spawned(self, _scope: &mut Scope<Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }

    fn timeout(self, _scope: &mut Scope<Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }

    fn wakeup(self, _scope: &mut Scope<Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }
}

impl<S: StreamSocket> Accepted for Machine<S> {
    type Seed = Seed;
    type Socket = S;

    fn accepted(socket: Self::Socket,
                seed: Seed,
                scope: &mut Scope<Context>)
                -> Response<Self, Void> {
        match scope.register(&socket, EventSet::hup() | EventSet::readable(), PollOpt::edge()) {
            Ok(()) => (),
            Err(err) => return Response::error(err.into()),
        }

        Response::ok(Machine {
            socket: socket,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
            functions: seed,
        })
    }
}
