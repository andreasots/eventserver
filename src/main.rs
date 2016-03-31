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
extern crate systemd;
extern crate log;
extern crate url;

macro_rules! log {
    ($level:expr, $($args:tt)*) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            __line: line!(),
            __file: file!(),
            __module_path: module_path!()
        };
        ::systemd::journal::log($level, &LOC, &format_args!($($args)*))
    });
}

macro_rules! error {
    ($($args:tt)*) => (log!(3, $($args)*));
}

macro_rules! info {
    ($($args:tt)*) => (log!(6, $($args)*));
}

macro_rules! debug {
    ($($args:tt)*) => (log!(7, $($args)*));
}

macro_rules! log_result {
    ($e:expr) => (match $e {
        ::std::result::Result::Ok(o) => ::std::result::Result::Ok(o),
        ::std::result::Result::Err(err) => {
            error!("`{}` failed: {}", stringify!($e), err);
            ::std::result::Result::Err(err)
        }
    })
}

macro_rules! nonblock {
    ($e:expr) => (match $e {
        ::std::result::Result::Ok(o) => ::std::result::Result::Ok(Some(o)),
        ::std::result::Result::Err(ref err) if err.kind() == ::std::io::ErrorKind::WouldBlock => ::std::result::Result::Ok(None),
        ::std::result::Result::Err(err) => ::std::result::Result::Err(err),
    })
}

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
    systemd::journal::JournalLog::init().unwrap();

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
