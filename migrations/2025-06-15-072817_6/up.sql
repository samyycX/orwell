-- Your SQL goes here
ALTER TABLE `clients_` DROP COLUMN `mceliece_pk_`;
ALTER TABLE `clients_` ADD COLUMN `kyber_pk_` BINARY NOT NULL;


