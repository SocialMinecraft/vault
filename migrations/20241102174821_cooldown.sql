-- Add migration script here

ALTER TABLE items ADD COLUMN cooldown TIMESTAMP;
