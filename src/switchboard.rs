use rotor::{Machine, Scope, Response, Void};
use rotor::mio::EventSet;

use super::Context;

pub struct Seed;

pub struct Fsm;

impl Machine for Fsm {
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
