-- This file should undo anything in `up.sql`

ALTER TABLE `messages_` ADD COLUMN `timestamp_` BIGINT NOT NULL;

