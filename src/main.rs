#![feature(custom_derive, plugin)]
#![plugin(serde_macros, diesel_codegen)]

#[macro_use]
extern crate rotor;
extern crate rotor_http;
extern crate rotor_stream;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate systemd;
extern crate log;
extern crate url;
#[macro_use]
extern crate diesel;
extern crate clap;
extern crate mime;
extern crate multipart;

use context::Context;

#[macro_use]
mod macros;
pub mod context;
pub mod http;
pub mod models;
pub mod rpc;
pub mod rpc_methods;
pub mod schema;

rotor_compose! {
    pub enum Fsm/Seed<Context> {
        Http(http::Fsm<rotor::mio::unix::UnixListener>),
        Rpc(rpc::Fsm<rotor::mio::unix::UnixListener>),
    }
}

pub fn main() {
    systemd::journal::JournalLog::init().unwrap();

    let matches = clap::App::new("Server-sent events server")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::Arg::with_name("rpc-socket")
            .long("rpc-socket")
            .value_name("PATH")
            .help("Path to RPC server socket")
            .takes_value(true)
            .required(true))
        .arg(clap::Arg::with_name("http-socket")
            .long("http-socket")
            .value_name("PATH")
            .help("Path to HTTP server socket")
            .takes_value(true)
            .required(true))
        .arg(clap::Arg::with_name("database-url")
            .long("database-url")
            .value_name("URL")
            .help("Postgres connection string")
            .takes_value(true)
            .required(true))
        .get_matches();

    let rpc_socket = rotor::mio::unix::UnixListener::bind(matches.value_of_os("rpc-socket").unwrap()).unwrap();
    let http_socket = rotor::mio::unix::UnixListener::bind(matches.value_of_os("http-socket").unwrap()).unwrap();

    let mut functions = std::collections::HashMap::new();
    functions.insert(String::from("send_event"), Box::new(rpc_methods::send_event) as Box<_>);
    functions.insert(String::from("register_key"), Box::new(rpc_methods::register_key) as Box<_>);
    let functions = std::rc::Rc::new(std::cell::RefCell::new(functions));

    let config = rotor::Config::new();
    let mut loop_ = rotor::Loop::new(&config).unwrap();

    loop_.add_machine_with(|scope| http::Fsm::new(http_socket, http::Seed, scope).wrap(Fsm::Http))
         .unwrap();
    loop_.add_machine_with(|scope| rpc::new(rpc_socket, functions, scope).wrap(Fsm::Rpc)).unwrap();

    loop_.instantiate(Context::new(matches.value_of("database-url").unwrap()).unwrap())
        .run().unwrap();
}
