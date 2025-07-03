-- Your SQL goes here

DROP TABLE IF EXISTS `keys_`;

CREATE TABLE `message_keys_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`msg_id_` TEXT NOT NULL,
	`receiver_id_` TEXT NOT NULL,
	`data_` BINARY NOT NULL
);

