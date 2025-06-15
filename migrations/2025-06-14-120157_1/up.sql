-- Your SQL goes here
CREATE TABLE `clients_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`name_` TEXT NOT NULL,
	`mceliece_pk_` BINARY NOT NULL,
	`dilithium_pk_` BINARY NOT NULL,
	`online_time_` BIGINT NOT NULL,
	`color_` INTEGER NOT NULL
);

CREATE TABLE `messages_`(
	`unique_id_` TEXT NOT NULL PRIMARY KEY,
	`msg_id_` TEXT NOT NULL,
	`sender_id_` INTEGER NOT NULL,
	`receiver_id_` INTEGER NOT NULL,
	`msg_type_` INTEGER NOT NULL,
	`data_` BINARY NOT NULL,
	`timestamp_` BIGINT NOT NULL
);

