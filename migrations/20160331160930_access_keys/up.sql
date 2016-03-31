CREATE TABLE access_keys (
    id SERIAL PRIMARY KEY,
    endpoint TEXT NOT NULL,
    key TEXT NOT NULL
);

CREATE UNIQUE INDEX access_keys_idx ON access_keys (endpoint, key);
