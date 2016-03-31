CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    endpoint TEXT NOT NULL,
    event TEXT NOT NULL,
    data TEXT NOT NULL
);
