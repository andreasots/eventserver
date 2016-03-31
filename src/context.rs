use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel;
use super::models::{Event, NewEvent, NewAccessKey};
use super::schema::{access_keys, events};
use std::rc::Rc;
use rotor::Notifier;
use std::sync::mpsc::Sender;

pub struct Context {
    pg_conn: PgConnection,
    clients: Vec<(Notifier, Sender<Rc<Event>>)>,
}

impl Context {
    pub fn new(database_url: &str) -> Result<Context, ConnectionError> {
        Ok(Context {
            pg_conn: try!(PgConnection::establish(database_url)),
            clients: vec![],
        })
    }

    pub fn send_event(&mut self, endpoint: String, event: String, data: String) -> Result<(), TransactionError<diesel::result::Error>> {
        let new_event = NewEvent {
            endpoint: endpoint,
            event: event,
            data: data,
        };

        let event = Rc::new(try!(self.pg_conn.transaction(||
            diesel::insert(&new_event)
                .into(events::table)
                .get_result::<Event>(&self.pg_conn)
        )));

        self.clients.retain(|&(ref notifier, ref channel)| {
            let _ = notifier.wakeup();
            channel.send(event.clone()).is_ok()
        });

        Ok(())
    }

    pub fn register_key(&mut self, endpoint: String, key: String) -> Result<(), TransactionError<diesel::result::Error>> {
        let new_access_key = NewAccessKey {
            endpoint: endpoint,
            key: key
        };
        self.pg_conn.transaction(||
            diesel::insert(&new_access_key)
                .into(access_keys::table)
                .execute(&self.pg_conn)
                .map(drop)
        )
    }

    pub fn check_key(&mut self, endpoint: &str, key: &str) -> Result<bool, diesel::result::Error> {
        Ok(try!(access_keys::dsl::access_keys.filter(access_keys::dsl::endpoint.eq(endpoint).and(access_keys::dsl::key.eq(key))).count().first::<i64>(&self.pg_conn)) != 0)
    }

    pub fn register_client(&mut self, notifier: Notifier, channel: Sender<Rc<Event>>) {
        self.clients.push((notifier, channel))
    }

    pub fn get_missed_events(&mut self, endpoint: &str, id: i64) -> Result<Vec<Event>, diesel::result::Error> {
        events::dsl::events.filter(events::dsl::endpoint.eq(endpoint).and(events::dsl::id.gt(id)))
            .load::<Event>(&self.pg_conn)
    }
}
