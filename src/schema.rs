table! {
    events {
        id -> BigInt,
        endpoint -> Text,
        event -> Text,
        data -> Text,
    }
}

table! {
    access_keys {
        id -> Integer,
        endpoint -> Text,
        key -> Text,
    }
}
