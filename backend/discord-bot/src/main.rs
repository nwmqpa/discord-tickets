use once_cell::sync::Lazy;
use serde::Deserialize;
use serenity::async_trait;
use serenity::prelude::*;

static CONFIG: Lazy<Config> = Lazy::new(|| {
    match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(error) => panic!("{:#?}", error)
    }
});

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "public_key")]
    _public_key: String,
    #[serde(rename = "app_id")]
    _app_id: u64,
    discord_token: String
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    
    let mut client = Client::builder(&CONFIG.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
