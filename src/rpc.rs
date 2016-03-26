use rotor_stream::{Accept, Accepted, Buf, StreamSocket};
use rotor::{Scope, Response, Void, GenericScope, Evented, EventSet, PollOpt};
use rotor::void::unreachable;
use rotor::mio::{TryAccept, TryRead, TryWrite};
use super::{msgpack, Context};
use std::any::Any;
use std::io::ErrorKind;
use nom::IResult;

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
    ($e:expr) => (match $e.unwrap() {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(e) => return ::rotor::Response::error(e.into()),
    });
}

macro_rules! ensure_bytes {
    ($buffer:expr, $n:expr, $error:expr) => (if $buffer.len() < $n { $error });
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
                        match msgpack::parse_array_length(&self.buffer[..]) {
                            IResult::Done(input, length) => {
                                let buffer_len = self.buffer.len();
                                self.buffer.consume(buffer_len - input.len());
                                State::Type { length: length }
                            }
                            IResult::Incomplete(_) => continue 'read,
                            IResult::Error(err) => return Response::error(err.into()),
                        }
                    }
                    State::Type { length } => unimplemented!(),
                    State::Request(RequestState::Msgid) => unimplemented!(),
                    State::Request(RequestState::Name { msgid }) => unimplemented!(),
                    State::Request(RequestState::Args { msgid, name }) => unimplemented!(),
                    State::Notify(NotifyState::Name) => unimplemented!(),
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
