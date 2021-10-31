table! {
    sounds (id) {
        id -> Text,
        name -> Text,
        extension -> Text,
        file_name -> Text,
        file_hash -> Text,
    }
}

table! {
    tags (id, sound_id, slug) {
        id -> Text,
        sound_id -> Text,
        slug -> Text,
    }
}

joinable!(tags -> sounds (sound_id));

allow_tables_to_appear_in_same_query!(sounds, tags,);
