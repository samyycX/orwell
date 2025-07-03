-- Your SQL goes here
CREATE TABLE `messages_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`sender_id_` TEXT NOT NULL,
	`data_` BINARY NOT NULL,
	`timestamp_` BIGINT NOT NULL
);

CREATE TABLE `keys_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`msg_id_` TEXT NOT NULL,
	`receiver_id_` TEXT NOT NULL,
	`data_` BINARY NOT NULL
);

CREATE TABLE `clients_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`name_` TEXT NOT NULL,
	`kyber_pk_` BINARY NOT NULL,
	`dilithium_pk_` BINARY NOT NULL,
	`online_time_` BIGINT NOT NULL,
	`color_` INTEGER NOT NULL
);

