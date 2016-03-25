#[macro_use]
extern crate rotor;
extern crate rotor_http;
extern crate rotor_stream;

mod http;
mod rpc;
mod switchboard;

pub struct Context;

rotor_compose! {
    pub enum Fsm/Seed<Context> {
        Http(http::Fsm<rotor::mio::unix::UnixListener>),
        Rpc(rpc::Fsm<rotor::mio::unix::UnixListener>),
        Switchboard(switchboard::Fsm),
    }
}

fn main() {
    let http_socket = rotor::mio::unix::UnixListener::bind("/run/eventserver-http").unwrap();
    let rpc_socket = rotor::mio::unix::UnixListener::bind("/run/eventserver-rpc").unwrap();

    let config = rotor::Config::new();
    let mut loop_creator = rotor::Loop::new(&config).unwrap();
    loop_creator.add_machine_with(|scope| {
        http::Fsm::new(http_socket, http::Seed, scope).wrap(Fsm::Http)
    }).unwrap();
    loop_creator.add_machine_with(|scope| {
        rotor_stream::Accept::<rpc::Machine<rotor::mio::unix::UnixStream>, rotor::mio::unix::UnixListener>::new(rpc_socket, rpc::Seed, scope).wrap(Fsm::Rpc)
    }).unwrap();
    loop_creator.add_machine_with(|scope| {
        rotor::Response::ok(Fsm::Switchboard(switchboard::Fsm))
    }).unwrap();
    let context = Context;
    let mut inst = loop_creator.instantiate(context);
    inst.run().unwrap();
}
