-- This file should undo anything in `up.sql`
ALTER TABLE `clients_` DROP COLUMN `kyber_pk_`;
ALTER TABLE `clients_` ADD COLUMN `mceliece_pk_` BINARY NOT NULL;


