-- This file should undo anything in `up.sql`

CREATE TABLE `keys_`(
	`id_` TEXT NOT NULL PRIMARY KEY,
	`msg_id_` TEXT NOT NULL,
	`receiver_id_` TEXT NOT NULL,
	`data_` BINARY NOT NULL
);


DROP TABLE IF EXISTS `message_keys_`;
