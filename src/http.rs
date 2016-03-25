use rotor_http::server::Fsm as HttpFsm;
use rotor_http::server::{Head, Server, Response, RecvMode};
use rotor::{Scope, Time};

use super::Context;

#[derive(Clone)]
pub struct Seed;

pub type Fsm<L> = HttpFsm<Machine, L>;

pub enum Machine {

}

impl Server for Machine {
    type Context = Context;
    type Seed = Seed;

    fn headers_received(seed: Self::Seed, head: Head, response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<(Self, RecvMode, Time)> {
        unimplemented!()
    }

    fn request_received(self, data: &[u8], response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<Self> {
        unimplemented!()
    }

    fn request_chunk(self, chunk: &[u8], response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<Self> {
        unimplemented!()
    }

    fn request_end(self, response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<Self> {
        unimplemented!()
    }

    fn timeout(self, response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<(Self, Time)> {
        unimplemented!()
    }

    fn wakeup(self, response: &mut Response, scope: &mut Scope<Self::Context>) -> Option<Self> {
        unimplemented!()
    }
}
