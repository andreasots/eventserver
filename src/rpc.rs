use rotor_stream::{Accept, Accepted, Buf, StreamSocket};
use rotor::{Scope, Response, Void, GenericScope, Evented, EventSet, PollOpt};
use rotor::void::unreachable;
use rotor::mio::{TryAccept, TryRead, TryWrite};
use super::{msgpack, Context};
use std::any::Any;
use std::io::ErrorKind;
use nom::IResult;
use std::u32;

pub type Fsm<L> = Accept<Machine<<L as TryAccept>::Output>, L>;

pub fn new<L: TryAccept + Evented + Any, S: GenericScope>(listener: L,
                                                          seed: Seed,
                                                          scope: &mut S)
                                                          -> Response<Fsm<L>, Void>
    where L::Output: StreamSocket
{
    Fsm::new(listener, seed, scope)
}

macro_rules! rotor_try {
    ($e:expr) => (match $e {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(e) => {
            error!("Called `rotor_try!` on an `Err` value: {}", e);
            return ::rotor::Response::error(e.into());
        },
    });
}

macro_rules! nom_try {
    ($e:expr, $on_incomplete:expr) => (match $e {
        ::nom::IResult::Done(input, value) => (input, value),
        ::nom::IResult::Incomplete(_) => $on_incomplete,
        ::nom::IResult::Error(err) => {
            let err = format!("{}", err);
            error!("Parse error: {}", err);
            return ::rotor::Response::error(err.into());
        }
    })
}

#[derive(Clone)]
pub struct Seed;

#[derive(Debug)]
enum RequestState {
    Msgid,
    Name {
        msgid: u32,
    },
    Args {
        msgid: u32,
        name: String,
    },
}

#[derive(Debug)]
enum NotifyState {
    Name,
    Args {
        name: String,
    },
}

#[derive(Debug)]
enum State {
    ArrayLength,
    Type {
        length: u32,
    },
    Request(RequestState),
    Notify(NotifyState),
}

pub struct Machine<S> {
    socket: S,
    buffer: Buf,
    state: State,
}

impl<S: StreamSocket> ::rotor::Machine for Machine<S> {
    type Context = Context;
    type Seed = Void;

    fn create(seed: Self::Seed, _scope: &mut Scope<Self::Context>) -> Response<Self, Void> {
        unreachable(seed)
    }

    fn ready(mut self,
             events: EventSet,
             scope: &mut Scope<Self::Context>)
             -> Response<Self, Self::Seed> {
        'read: loop {
            match self.buffer.read_from(&mut self.socket) {
                Ok(_) => (),
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Response::error(err.into()),
            }
            loop {
                let new_state = match self.state {
                    State::ArrayLength => {
                        let (length, consumed) = {
                            let (input, length) = nom_try!(msgpack::parse_array_length(&self.buffer[..]), continue 'read);
                            (length, self.buffer.len() - input.len())
                        };
                        self.buffer.consume(consumed);
                        State::Type { length: length }
                    }
                    State::Type { length } => {
                        let (ty, consumed) = {
                            let (input, ty) = nom_try!(msgpack::parse_integer(&self.buffer[..]), continue 'read);
                            (ty, self.buffer.len() - input.len())
                        };
                        self.buffer.consume(consumed);
                        match ty {
                            msgpack::Integer::Unsigned(0) | msgpack::Integer::Signed(0) => if length == 4 { State::Request(RequestState::Msgid) } else { unimplemented!() },
                            msgpack::Integer::Unsigned(2) | msgpack::Integer::Signed(2) => if length == 3 { State::Notify(NotifyState::Name) } else { unimplemented!() },
                            msgpack::Integer::Unsigned(n) => {
                                error!("Unrecognised message type {}", n);
                                return Response::error(format!("Unrecognised message type {}", n).into());
                            },
                            msgpack::Integer::Signed(n) => {
                                error!("Unrecognised message type {}", n);
                                return Response::error(format!("Unrecognised message type {}", n).into());
                            },
                        }
                    },
                    State::Request(RequestState::Msgid) => {
                        let (ty, consumed) = {
                            let (input, ty) = nom_try!(msgpack::parse_integer(&self.buffer[..]), continue 'read);
                            (ty, self.buffer.len() - input.len())
                        };
                        self.buffer.consume(consumed);
                        match ty {
                            msgpack::Integer::Unsigned(msgid) if msgid < u32::MAX as u64 => State::Request(RequestState::Name { msgid: msgid as u32 }),
                            msgpack::Integer::Signed(msgid) if msgid >= 0 && msgid < u32::MAX as i64 => State::Request(RequestState::Name { msgid: msgid as u32 }),
                            msgpack::Integer::Unsigned(n) => {
                                error!("Message ID {} is out of range.", n);
                                return Response::error(format!("Message ID {} is out of range.", n).into());
                            },
                            msgpack::Integer::Signed(n) => {
                                error!("Message ID {} is out of range.", n);
                                return Response::error(format!("Message ID {} is out of range.", n).into());
                            },
                        }
                    },
                    State::Request(RequestState::Name { msgid }) => {
                        let (name, consumed) = {
                            let (input, name) = nom_try!(msgpack::parse_string(&self.buffer[..]), continue 'read);
                            (name, self.buffer.len() - input.len())
                        };
                        self.buffer.consume(consumed);
                        State::Request(RequestState::Args { msgid: msgid, name: name })
                    },
                    State::Request(RequestState::Args { msgid, name }) => unimplemented!(),
                    State::Notify(NotifyState::Name) => {
                        let (name, consumed) = {
                            let (input, name) = nom_try!(msgpack::parse_string(&self.buffer[..]), continue 'read);
                            (name, self.buffer.len() - input.len())
                        };
                        self.buffer.consume(consumed);
                        State::Notify(NotifyState::Args { msgid: msgid, name: name })
                    },
                    State::Notify(NotifyState::Args { name }) => unimplemented!(),
                };

                println!("{:?} -> {:?}", self.state, new_state);
                self.state = new_state;
            }
        }

        Response::ok(self)
    }

    fn spawned(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }

    fn timeout(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }

    fn wakeup(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        Response::ok(self)
    }
}

impl<S: StreamSocket> Accepted for Machine<S> {
    type Seed = Seed;
    type Socket = S;

    fn accepted(socket: Self::Socket,
                seed: Seed,
                scope: &mut Scope<Self::Context>)
                -> Response<Self, Void> {
        match scope.register(&socket, EventSet::readable(), PollOpt::edge()) {
            Ok(()) => (),
            Err(err) => return Response::error(err.into()),
        }

        Response::ok(Machine {
            socket: socket,
            buffer: Buf::new(),
            state: State::ArrayLength,
        })
    }
}
