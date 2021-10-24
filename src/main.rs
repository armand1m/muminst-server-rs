mod app_state;
mod discord;
mod handlers;

use dotenv;
use songbird::SerenityInit;
use std::env;

use serenity::{client::Client, framework::StandardFramework};

use actix_web::{web, App, HttpServer};
use app_state::AppState;
use discord::{commands::BOTCOMMANDS_GROUP, DiscordHandler};
use handlers::index;

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

    let mut client = Client::builder(&token)
        .event_handler(DiscordHandler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|reason| eprintln!("Discord Client ended: {:?}", reason));
    });

    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(web::Data::new(AppState {
                app_name: String::from("muminst-server-rust"),
            }))
            .service(index)
    })
    .bind("0.0.0.0:8080")
    .expect("Failed to bind server to 0.0.0.0:8080")
    .run()
    .await
    .unwrap();

    tokio::signal::ctrl_c().await.unwrap();

    println!("Received Ctrl-C, shutting down.");
}
