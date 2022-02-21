#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate diesel;

mod actions;
mod app_state;
mod discord;
mod handlers;
pub mod models;
pub mod schema;
mod websocket;

use diesel_migrations::run_pending_migrations;
use dotenv;
use log::info;
use songbird::SerenityInit;
use std::{
    env,
    sync::{Arc, Mutex},
};

use serenity::{
    client::{Client, Context},
    framework::StandardFramework,
};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::rt::System;
use actix_web::{middleware::Logger, web, web::Data, App, HttpServer};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tokio::runtime::Builder;

use app_state::AppState;
use discord::{commands::BOTCOMMANDS_GROUP, DiscordHandler};
use handlers::{
    add_tags::add_tags_handler, play_sound::play_sound_handler, sounds::sounds_handler,
    upload::upload_handler,
};
use websocket::sound_lock::sound_lock_handler;

lazy_static! {
    pub static ref DISCORD_CTX: Arc<Mutex<Option<Context>>> = Arc::new(Mutex::new(None));
}

fn main() {
    dotenv::dotenv().ok();

    let logger_env = env_logger::Env::new().default_filter_or("info,tracing::span=off");
    env_logger::init_from_env(logger_env);

    let thread_count = env::var("THREAD_COUNT")
        .unwrap_or(1.to_string())
        .parse::<usize>()
        .expect("THREAD_COUNT env var should be a valid number");

    System::with_tokio_rt(|| {
        Builder::new_multi_thread()
            .enable_all()
            .worker_threads(thread_count)
            .build()
            .unwrap()
    })
    .block_on(async_main());
}

async fn async_main() {
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN to be set in the environment");
    let discord_guild_id = env::var("DISCORD_GUILD_ID")
        .expect("DISCORD_GUILD_ID to be set in the environment")
        .parse::<u64>()
        .expect("DISCORD_GUILD_ID should be a valid number");
    let database_path =
        env::var("DATABASE_PATH").expect("DATABASE_PATH to be set in the environment");
    let audio_folder_path =
        env::var("AUDIO_PATH").expect("AUDIO_PATH to be set in the environment");
    let should_run_pending_migrations = env::var("RUN_PENDING_MIGRATIONS")
        .unwrap_or("false".to_string())
        .parse::<bool>()
        .expect("RUN_PENDING_MIGRATIONS should be a boolean");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&BOTCOMMANDS_GROUP);

    let event_handler = DiscordHandler {};

    let mut client = Client::builder(&token)
        .event_handler(event_handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Discord client instance to be created.");

    let discord_client_thread = tokio::spawn(async move {
        client
            .start()
            .await
            .map_err(|reason| eprintln!("Discord client connection was terminated: {:?}", reason))
    });

    let manager = ConnectionManager::<SqliteConnection>::new(database_path);
    let database_pool = Pool::builder().max_size(10).build(manager).unwrap();

    if should_run_pending_migrations {
        let database_connection = database_pool
            .get()
            .expect("Failed to acquire db connection from db pool");
        run_pending_migrations(&database_connection).expect("Failed to run pending migrations.");
    }

    let http_server_thread = HttpServer::new(move || {
        let app_name = "muminst-server-rust".to_string();
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method();

        let logger = Logger::default();

        App::new()
            .wrap(cors)
            .wrap(logger)
            .app_data(Data::new(AppState {
                app_name,
                discord_guild_id,
                discord_ctx: DISCORD_CTX.to_owned(),
                database_pool: database_pool.clone(),
                audio_folder_path: audio_folder_path.clone(),
            }))
            .service(web::resource("/ws").to(sound_lock_handler))
            .service(sounds_handler)
            .service(upload_handler)
            .service(play_sound_handler)
            .service(add_tags_handler)
            .service(Files::new("/assets", "./data/audio"))
    })
    .bind("0.0.0.0:8080")
    .expect("Failed to bind http server to 0.0.0.0:8080")
    .run();

    /*
     * This handles terminating all threads in case
     * one of them gets terminated/finished.
     */
    tokio::select! {
        _ = discord_client_thread => 0,
        _ = http_server_thread => 0,
    };

    tokio::signal::ctrl_c().await.unwrap();

    info!("Received Ctrl-C, shutting down.");
}
