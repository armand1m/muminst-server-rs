-- Your SQL goes here
CREATE TABLE sounds (
	id TEXT NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	extension TEXT NOT NULL,
	file_name TEXT NOT NULL,
	file_hash TEXT NOT NULL
);
