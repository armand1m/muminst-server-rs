# Contributing

Thanks for wanting to contribute to the Muminst rust server.

## Preparing environment

Make sure you have the default rust tooling (`rustc` and `cargo`) setup.

Muminst depends on `ffmpeg` and `youtubedl`, make sure to have both installed on your environment.

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

```sh
cargo watch -x run
```

## Database

This application makes use of Diesel to manage an SQLite database.

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