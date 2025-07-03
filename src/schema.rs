// @generated automatically by Diesel CLI.

diesel::table! {
    clients_ (id_) {
        id_ -> Text,
        name_ -> Text,
        kyber_pk_ -> Binary,
        dilithium_pk_ -> Binary,
        online_time_ -> BigInt,
        color_ -> Integer,
    }
}

diesel::table! {
    message_keys_ (id_) {
        id_ -> Text,
        msg_id_ -> Text,
        receiver_id_ -> Text,
        data_ -> Binary,
    }
}

diesel::table! {
    messages_ (id_) {
        id_ -> Text,
        sender_id_ -> Text,
        data_ -> Binary,
        timestamp_ -> BigInt,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    clients_,
    message_keys_,
    messages_,
);
