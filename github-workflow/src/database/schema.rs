table! {
    config (key) {
        key -> Text,
        value -> Text,
    }
}

table! {
    repositories (name_with_owner) {
        name_with_owner -> Text,
        name -> Text,
        url -> Text,
        pushed_at -> Timestamp,
    }
}

allow_tables_to_appear_in_same_query!(
    config,
    repositories,
);
