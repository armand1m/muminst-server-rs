# Contributing

Thanks for wanting to contribute to the Muminst rust server.

## Preparing environment

Make sure you have the default rust tooling (`rustc` and `cargo`) setup.

Muminst depends on `opus`, `ffmpeg` and `youtubedl`. Make sure to have these installed on your environment.

There is a `Dockerfile` available that can be used to create a containerized version of the server.
Minor tweak can be done to enable a containerized development environment.

## .env file

Copy the `.env.example` file into a `.env` file and edit its content to contain your discord bot token and configuration.

```sh
cp ./.env.example ./.env
```
## Building

```sh
cargo build
```

## Running

```sh
cargo run
```

## Watch mode

Install `cargo-watch`:

```sh
cargo install cargo-watch
```

Then use the following command:

```sh
cargo watch -x run
```

## Database

This application makes use of Diesel to manage an SQLite database. Diesel is smart enough to allow us to switch to a postgres database when we see fit.

### Prepare project 

Run this from the root of the project:

```sh
mkdir -p ./data/audio
```

Set these in your .env file:

```sh
# (required) path to the sqlite database in the current host. the application will create and seed the db in case it does not exist.
DATABASE_PATH=./data/database.db 

# (required) path to the stored audios. will be used for both playing and uploads.
AUDIO_PATH=./data/audio 
```

### Automatic migration and database setup

Make sure to set the following env var in your `.env` file:

```sh
# (optional, default = false) run pending database migrations during server startup
RUN_PENDING_MIGRATIONS=true
```

Now just run `cargo run` and you should have the app with a database up and running.

After that, disable `RUN_PENDING_MIGRATIONS` in your `.env` file in case you feel you don't want that to happen without you knowing about it.

### Manual migration and database setup

Run the following to install diesel cli, setup your machine and run the migrations.

The script will create an SQLite database in the path specified through `DATABASE_URL` environment variable in case it does not exist already.

```sh
cargo install diesel_cli --no-default-features --features sqlite
diesel setup
diesel migration run
```

Read Diesel CLI docs to learn more.

Migrations are meant to be committed to version control.

## Logs

Logging is handled by the `log` crate. 

The `RUST_LOG` env var can be set to configure which logs you wanna see based on the modules or level of information you're interested into.

By default, all info logs from all modules except by `tracing::span` are enabled. This is the same as running the server with `RUST_LOG=info,tracing::span=off` 
