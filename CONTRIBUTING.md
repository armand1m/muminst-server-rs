## Database

This application makes use of Diesel to manage an SQLite database.

Run the following to install diesel cli, setup your machine and run the migrations.

The script will create an SQLite database in the path specified through `DATABASE_URL` in case it does not exist already.

```sh
cargo install diesel_cli --no-default-features --features sqlite
diesel setup
diesel migration run
```

Read Diesel CLI docs to learn more.

Migrations are meant to be committed to version control.