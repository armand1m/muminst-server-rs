-- Your SQL goes here
CREATE TABLE sounds (
	id TEXT NOT NULL PRIMARY KEY,
	name TEXT NOT NULL,
	extension TEXT NOT NULL,
	fileName TEXT NOT NULL,
	fileHash TEXT NOT NULL
);
