use chrono::{DateTime, Local};
use diesel::{insert_into, prelude::*, sql_types::Timestamp};
use orwell::{
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
    pub unique_id_: String,
    pub msg_id_: String,
    pub msg_type_: i32,
    pub sender_id_: String,
    pub receiver_id_: String,
    pub data_: Vec<u8>,
    pub timestamp_: i64,
}

pub struct MessageManager {}

impl MessageManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn add_message(
        sender_id: String,
        msg_id: String,
        msg_type: i32,
        receiver_id: String,
        data: Vec<u8>,
    ) {
        let unique_id = Uuid::now_v7().to_string();
        let mut conn: SqliteConnection = get_db_connection();
        let message = Message {
            unique_id_: unique_id,
            msg_id_: msg_id,
            sender_id_: sender_id,
            receiver_id_: receiver_id,
            msg_type_: msg_type,
            data_: data,
            timestamp_: get_now_timestamp() as i64,
        };
        insert_into(messages_)
            .values(message)
            .execute(&mut conn)
            .unwrap();
    }

    pub async fn get_history_messages(receiver_id: String, amount: i32) -> Vec<Message> {
        let mut conn: SqliteConnection = get_db_connection();
        messages_
            .filter(receiver_id_.eq(receiver_id))
            .order_by(timestamp_.desc())
            .limit(amount as i64)
            .load::<Message>(&mut conn)
            .unwrap()
    }
}
