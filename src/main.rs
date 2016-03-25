#[macro_use]
extern crate rotor;
extern crate rotor_http;
extern crate rotor_stream;
extern crate xdg_basedir;

mod http;
mod rpc;

pub struct Context;

rotor_compose! {
    pub enum Fsm/Seed<Context> {
        Http(http::Fsm<rotor::mio::unix::UnixListener>),
        Rpc(rpc::Fsm<rotor::mio::unix::UnixListener>),
    }
}

fn main() {
    let mut socket_path = xdg_basedir::get_runtime_dir().expect("$XDG_RUNTIME_DIR unset");

    socket_path.push("eventserver-http");
    let http_socket = rotor::mio::unix::UnixListener::bind(&socket_path).unwrap();
    socket_path.pop();
    socket_path.push("eventserver-rpc");
    let rpc_socket = rotor::mio::unix::UnixListener::bind(&socket_path).unwrap();
    socket_path.pop();

    let config = rotor::Config::new();
    let mut loop_creator = rotor::Loop::new(&config).unwrap();

    loop_creator.add_machine_with(|scope| {
        http::Fsm::new(http_socket, http::Seed, scope).wrap(Fsm::Http)
    }).unwrap();
    loop_creator.add_machine_with(|scope| {
        rpc::new(rpc_socket, rpc::Seed, scope).wrap(Fsm::Rpc)
    }).unwrap();

    let context = Context;

    loop_creator.instantiate(context).run().unwrap();
}
