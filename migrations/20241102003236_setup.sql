-- Add migration script here

CREATE TABLE  players (
    player  UUID PRIMARY KEY,
    created timestamp NOT NULL DEFAULT current_timestamp
);

CREATE TABLE items (
    player UUID NOT NULL references players(player),
    slot   int NOT NULL,

    type       VARCHAR(100) NOT NULL,
    amount     INT NOT NULL,
    durability INT NOT NULL,

    display_name      TEXT,
    custom_model_data INT,
    lore              TEXT[] NOT NULL,
    enchants          TEXT[] NOT NULL,
    flags             TEXT[] NOT NULL,

    PRIMARY KEY (player, slot)
);