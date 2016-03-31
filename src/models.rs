use super::schema::{access_keys, events};

#[derive(Queryable)]
pub struct AccessKey {
    pub id: i32,
    pub endpoint: String,
    pub key: String,
}

#[derive(Queryable)]
pub struct Event {
    pub id: i64,
    pub endpoint: String,
    pub event: String,
    pub data: String,
}

#[insertable_into(access_keys)]
pub struct NewAccessKey {
    pub endpoint: String,
    pub key: String,
}

#[insertable_into(events)]
pub struct NewEvent {
    pub endpoint: String,
    pub event: String,
    pub data: String,
}
