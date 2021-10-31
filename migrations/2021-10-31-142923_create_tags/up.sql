-- Your SQL goes here
CREATE TABLE tags (
	id TEXT NOT NULL UNIQUE,
    soundId TEXT NOT NULL,
    slug TEXT NOT NULL,
    PRIMARY KEY (id, soundId, slug),
    FOREIGN KEY (soundId) 
        REFERENCES sounds (id) 
            ON DELETE CASCADE 
            ON UPDATE NO ACTION
);