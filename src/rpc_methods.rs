use serde_json::{self, Value};
use rotor::Scope;
use super::context::Context;

macro_rules! try_rpc {
    ($e:expr) => (match log_result!($e) {
        ::std::result::Result::Ok(o) => o,
        ::std::result::Result::Err(e) => return ::std::result::Result::Err(::serde_json::Value::String(format!("{}", e))),
    })
}

pub fn send_event(scope: &mut Scope<Context>, param: Value, _user: Option<i32>) -> Result<Value, Value> {
    let (endpoint, event, data) = try_rpc!(serde_json::from_value::<(String, String, String)>(param));
    try_rpc!(scope.send_event(endpoint, event, data));
    Ok(Value::Null)
}

pub fn register_key(scope: &mut Scope<Context>, param: Value, _user: Option<i32>) -> Result<Value, Value> {
    let (endpoint, key) = try_rpc!(serde_json::from_value::<(String, String)>(param));
    try_rpc!(scope.register_key(endpoint, key));
    Ok(Value::Null)
}
