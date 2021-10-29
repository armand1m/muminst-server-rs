mod app_state;
mod discord;
mod handlers;

#[macro_use]
extern crate lazy_static;

use dotenv;
use songbird::SerenityInit;
use std::{
    env,
    sync::{Arc, Mutex},
};

use serenity::{
    client::{Client, Context},
    framework::StandardFramework,
};

use actix_web::{web, App, HttpServer};
use app_state::AppState;
use discord::{commands::BOTCOMMANDS_GROUP, DiscordHandler};
use handlers::{index, play_sound};

lazy_static! {
    pub static ref DISCORD_CTX: Arc<Mutex<Option<Context>>> = Arc::new(Mutex::new(None));
}

fn main() {
    dotenv::dotenv().ok();
    actix_web::rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(8)
            .thread_name("main-tokio")
            .build()
            .unwrap()
    })
    .block_on(async_main());
}

async fn async_main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a discord token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~"))
        .group(&BOTCOMMANDS_GROUP);

    let event_handler = DiscordHandler {};

    let mut client = Client::builder(&token)
        .event_handler(event_handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error while creating discord client.");

    tokio::spawn(async move {
        client
            .start()
            .await
            .map_err(|reason| eprintln!("Discord client connection was terminated: {:?}", reason))
    });

    HttpServer::new(move || {
        let app_name = String::from("muminst-server-rust");

        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(web::Data::new(AppState {
                app_name,
                discord_ctx: DISCORD_CTX.to_owned(),
            }))
            .service(index)
            .service(play_sound)
    })
    .bind("0.0.0.0:8080")
    .expect("Failed to bind http server to 0.0.0.0:8080")
    .run()
    .await
    .unwrap();

    tokio::signal::ctrl_c().await.unwrap();

    println!("Received Ctrl-C, shutting down.");
}
