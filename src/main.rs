#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

#[macro_use]
extern crate rotor;
extern crate rotor_http;
extern crate rotor_stream;
extern crate xdg_basedir;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate log;

mod http;
mod rpc;

pub struct Context;

rotor_compose! {
    pub enum Fsm/Seed<Context> {
        Http(http::Fsm<rotor::mio::unix::UnixListener>),
        Rpc(rpc::Fsm<rotor::mio::unix::UnixListener>),
    }
}

fn send_event(scope: &mut rotor::Scope<Context>, param: serde_json::Value, user: Option<i32>) -> Result<serde_json::Value, serde_json::Value> {
    Err(serde_json::Value::String(String::from("not yet implemented")))
}

fn main() {
    let mut socket_path = xdg_basedir::get_runtime_dir().expect("$XDG_RUNTIME_DIR unset");

    socket_path.push("eventserver-http");
    let http_socket = rotor::mio::unix::UnixListener::bind(&socket_path).unwrap();
    socket_path.pop();
    socket_path.push("eventserver-rpc");
    let rpc_socket = rotor::mio::unix::UnixListener::bind(&socket_path).unwrap();
    socket_path.pop();

    let mut functions = std::collections::HashMap::new();
    functions.insert(String::from("send_event"), Box::new(send_event) as Box<_>);
    let functions = std::rc::Rc::new(std::cell::RefCell::new(functions));

    let config = rotor::Config::new();
    let mut loop_ = rotor::Loop::new(&config).unwrap();

    loop_.add_machine_with(|scope| http::Fsm::new(http_socket, http::Seed, scope).wrap(Fsm::Http))
         .unwrap();
    loop_.add_machine_with(|scope| rpc::new(rpc_socket, functions, scope).wrap(Fsm::Rpc)).unwrap();

    let context = Context;

    loop_.instantiate(context).run().unwrap();
}
