#[macro_use]
extern crate diesel;

mod actions;
mod app_state;
mod discord;
mod handlers;
mod lock;
pub mod models;
pub mod schema;
mod websocket;

use actix::prelude::*;
use diesel_migrations::run_pending_migrations;
use log::{error, info};
use songbird::{SerenityInit, Songbird};
use std::env;

use serenity::{client::Client, framework::StandardFramework};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{middleware::Logger, web, web::Data, App, HttpServer};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;

use app_state::AppState;
use discord::{actor::DiscordActor, commands::BOTCOMMANDS_GROUP, DiscordHandler};
use handlers::{
    add_tags::add_tags_handler, play_sound::play_sound_handler, sounds::sounds_handler,
    upload::upload_handler,
};
use websocket::sound_lock::sound_lock_handler;

#[actix_web::main]
async fn main() {
    dotenv::dotenv().ok();

    let logger_env = env_logger::Env::new().default_filter_or("info,tracing::span=off");
    env_logger::init_from_env(logger_env);

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
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .expect("RUN_PENDING_MIGRATIONS should be a boolean");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&BOTCOMMANDS_GROUP);

    let event_handler = DiscordHandler {};

    let songbird = Songbird::serenity();
    let discord_actor_addr = DiscordActor {
        discord_guild_id,
        songbird: songbird.clone(),
    }
    .start();

    let mut client = Client::builder(&token)
        .event_handler(event_handler)
        .framework(framework)
        .register_songbird_with(songbird)
        .await
        .expect("Discord client instance to be created.");

    let shard_manager = client.shard_manager.clone();

    let discord_client_thread = actix_web::rt::spawn(async move {
        let _ = client
            .start_autosharded()
            .await
            .map_err(|reason| error!("Discord client connection was terminated: {:?}", reason));
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

        let app_data = Data::new(AppState {
            app_name,
            discord_guild_id,
            discord_actor_addr: discord_actor_addr.clone(),
            database_pool: database_pool.clone(),
            audio_folder_path: audio_folder_path.clone(),
        });
        let websocket_handler = web::resource("/ws").to(sound_lock_handler);

        App::new()
            .wrap(cors)
            .wrap(logger)
            .app_data(app_data)
            .service(websocket_handler)
            .service(sounds_handler)
            .service(upload_handler)
            .service(play_sound_handler)
            .service(add_tags_handler)
            .service(Files::new("/assets", audio_folder_path.clone()))
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

    shard_manager.lock().await.shutdown_all().await;

    info!("Received Ctrl-C, shutting down.");
}
