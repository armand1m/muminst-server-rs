-- Your SQL goes here
CREATE TABLE tags (
    id TEXT NOT NULL UNIQUE,
    sound_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    PRIMARY KEY (id, sound_id, slug),
    FOREIGN KEY (sound_id) 
        REFERENCES sounds (id) 
            ON DELETE CASCADE 
            ON UPDATE NO ACTION
);