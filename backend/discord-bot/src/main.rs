use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::delete;
use axum::routing::post;
use axum::Extension;
use axum::Json;
use axum::Router;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::json;
use serenity::async_trait;
use serenity::prelude::*;
use serenity::CacheAndHttp;
use tracing::log;

static CONFIG: Lazy<Config> = Lazy::new(|| match envy::from_env::<Config>() {
    Ok(config) => config,
    Err(error) => panic!("{:#?}", error),
});

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "public_key")]
    _public_key: String,
    #[serde(rename = "app_id")]
    _app_id: u64,
    discord_token: String,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Login with a bot token from the environment
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&CONFIG.discord_token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    let http = client.cache_and_http.clone();

    // build our application with a route
    let app = Router::new()
        .route("/ticket", post(create_ticket_channel))
        .route("/ticket", delete(delete_ticket_channel))
        .layer(Extension(http));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    log::debug!("listening on {}", addr);
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    tokio::select! {
        result = client.start() => {
            if let Err(why) = result {
                log::error!("An error occurred while running the client: {why:?}");
            }
        },
        result = server => {
            if let Err(why) = result {
                log::error!("An error occured while running the client: {why:?}");
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ticket {
    title: String,
    _content: String,
    channel_id: Option<u64>,
}

const GUILD_ID: u64 = 970763115903991908;

async fn create_ticket_channel(
    Extension(http): Extension<Arc<CacheAndHttp>>,
    Json(ticket): Json<Ticket>,
) -> impl IntoResponse {
    let kebabed_title = ticket.title.replace(" ", "-");

    let channel_settings = json!({
        "type": 0,
        "name": kebabed_title
    });

    let settings_map = channel_settings.as_object().expect("Object was just");

    let channel_creation = http
        .as_ref()
        .http
        .create_channel(GUILD_ID, settings_map, Some("ticket creation"))
        .await;

    if let Err(why) = channel_creation {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": why.to_string()
            })),
        )
    } else {
        let channel = channel_creation.unwrap();

        (
            StatusCode::OK,
            Json(json!({
                "channel_id": channel.id.0
            })),
        )
    }
}

async fn delete_ticket_channel(
    Extension(http): Extension<Arc<CacheAndHttp>>,
    Json(ticket): Json<Ticket>,
) -> impl IntoResponse {
    if let Some(channel_id) = ticket.channel_id {
        if let Err(why) = http.as_ref().http.delete_channel(channel_id).await {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": format!("Could not find chanel: {why:?}")
                })),
            )
        } else {
            (StatusCode::OK, Json(json!({})))
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Could not find 'channelId' property"
            })),
        )
    }
}
