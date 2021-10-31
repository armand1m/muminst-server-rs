table! {
    sounds (id) {
        id -> Text,
        name -> Text,
        extension -> Text,
        fileName -> Text,
        fileHash -> Text,
    }
}

table! {
    tags (id, soundId, slug) {
        id -> Text,
        soundId -> Text,
        slug -> Text,
    }
}

joinable!(tags -> sounds (soundId));

allow_tables_to_appear_in_same_query!(sounds, tags,);
