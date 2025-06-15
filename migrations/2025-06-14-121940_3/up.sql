-- Your SQL goes here

ALTER TABLE `messages_` DROP COLUMN `sender_id_`;
ALTER TABLE `messages_` DROP COLUMN `receiver_id_`;
ALTER TABLE `messages_` ADD COLUMN `sender_id_` TEXT NOT NULL;
ALTER TABLE `messages_` ADD COLUMN `receiver_id_` TEXT NOT NULL;

