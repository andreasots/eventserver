use rotor_stream::{Accept, Accepted, StreamSocket};
use rotor::{Scope, Response, Void, GenericScope};
use rotor::mio::{EventSet, TryAccept};
use super::Context;

pub type Fsm<L> = Accept<Machine<<L as TryAccept>::Output>, L>;

#[derive(Clone)]
pub struct Seed;

enum State {
}

pub struct Machine<S> {
    socket: S,
    state: State,
}

impl<S> ::rotor::Machine for Machine<S> {
    type Context = Context;
    type Seed = Seed;

    fn create(seed: Self::Seed, scope: &mut Scope<Self::Context>) -> Response<Self, Void> {
        unimplemented!()
    }

    fn ready(self, events: EventSet, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        unimplemented!()
    }

    fn spawned(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        unimplemented!()
    }

    fn timeout(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        unimplemented!()
    }

    fn wakeup(self, scope: &mut Scope<Self::Context>) -> Response<Self, Self::Seed> {
        unimplemented!()
    }
}

impl<S: StreamSocket> Accepted for Machine<S> {
    type Seed = Seed;
    type Socket = S;

    fn accepted(sock: Self::Socket, seed: Seed, scope: &mut Scope<Self::Context>) -> Response<Self, Void> {
        unimplemented!()
    }
}
