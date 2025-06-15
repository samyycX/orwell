-- This file should undo anything in `up.sql`

ALTER TABLE `messages_` DROP COLUMN `sender_id_`;
ALTER TABLE `messages_` DROP COLUMN `receiver_id_`;
ALTER TABLE `messages_` ADD COLUMN `sender_id_` INTEGER NOT NULL;
ALTER TABLE `messages_` ADD COLUMN `receiver_id_` INTEGER NOT NULL;

