use chrono::{DateTime, Local};
use diesel::{insert_into, prelude::*, sql_types::Timestamp};
use orwell::{
    pb::orwell::Key as PbKey,
    schema::message_keys_::{self, dsl::*},
    schema::messages_::{self, dsl::*},
    shared::helper::get_now_timestamp,
};
use uuid::Uuid;

use crate::get_db_connection;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = messages_)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(primary_key(unique_id_))]
pub struct Message {
    pub id_: String,
    pub sender_id_: String,
    pub data_: Vec<u8>,
    pub timestamp_: i64,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = message_keys_)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(primary_key(unique_id_))]
pub struct MessageKey {
    pub id_: String,
    pub msg_id_: String,
    pub receiver_id_: String,
    pub data_: Vec<u8>,
}

pub struct MessageManager {}

impl MessageManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn add_message(sender_id: String, data: Vec<u8>, keys: Vec<PbKey>) {
        let msg_id = Uuid::now_v7().to_string();
        let mut conn: SqliteConnection = get_db_connection();
        let message = Message {
            id_: msg_id.clone(),
            sender_id_: sender_id,
            data_: data,
            timestamp_: get_now_timestamp() as i64,
        };
        insert_into(messages_)
            .values(message)
            .execute(&mut conn)
            .unwrap();
        for key in keys {
            let id = Uuid::now_v7().to_string();
            let key = MessageKey {
                id_: id,
                msg_id_: msg_id.clone(),
                receiver_id_: key.receiver_id,
                data_: key.ciphertext,
            };
            insert_into(message_keys_)
                .values(key)
                .execute(&mut conn)
                .unwrap();
        }
    }

    pub async fn get_history_messages(
        receiver_id: String,
        amount: i32,
    ) -> Vec<(Message, MessageKey)> {
        let mut conn: SqliteConnection = get_db_connection();
        messages_::table
            .inner_join(message_keys_::table.on(message_keys_::msg_id_.eq(messages_::id_)))
            .filter(message_keys_::receiver_id_.eq(receiver_id))
            .order(messages_::timestamp_.desc())
            .limit(amount as i64)
            .load::<(Message, MessageKey)>(&mut conn)
            .unwrap()
    }
}
