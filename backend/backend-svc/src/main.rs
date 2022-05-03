use std::{net::SocketAddr, str::FromStr, sync::Arc};

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use futures::StreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::ClientOptions,
    results::DeleteResult,
    Client,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::log;

use crate::discord_bot::{DiscordBotAPI, DynDiscordBotAPI, ExternalDiscordBotAPI};

mod discord_bot;

static CONFIG: Lazy<Config> = Lazy::new(|| match envy::from_env::<Config>() {
    Ok(config) => config,
    Err(error) => panic!("{:#?}", error),
});

#[derive(Deserialize)]
struct Config {
    mongo_host: String,
    mongo_database: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    title: String,
    content: String,
    channel_id: Option<u64>,
}

impl Ticket {
    pub fn with_id(self) -> Self {
        Self {
            id: self.id.or_else(|| Some(ObjectId::new())),
            ..self
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut client_options = ClientOptions::parse(&CONFIG.mongo_host).await?;

    // Manually set an option.
    client_options.app_name = Some("Backend Service".to_string());

    // Get a handle to the deployment.
    let client = Client::with_options(client_options)?;

    let discord_api =
        Arc::new(ExternalDiscordBotAPI::with_url("http://localhost:3001")) as DynDiscordBotAPI;

    // build our application with a route
    let app = Router::new()
        .route("/tickets", get(get_tickets))
        .route("/tickets", post(add_ticket))
        .route("/tickets/:mongo_id", delete(delete_ticket))
        .layer(Extension(client))
        .layer(Extension(discord_api));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    log::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn delete_ticket(
    Extension(mongodb): Extension<Client>,
    Path(ticket_id): Path<String>,
    Extension(discord_bot): Extension<DynDiscordBotAPI>,
) -> impl IntoResponse {
    let result = mongodb
        .database(&CONFIG.mongo_database)
        .collection::<Ticket>("tickets")
        .find_one(
            doc! {
                "_id": ObjectId::from_str(&ticket_id).expect("Couldn't transform OID")
            },
            None,
        )
        .await;

    if let Err(why) = result {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": why.to_string() })),
        )
    } else {
        if let Some(ticket) = result.unwrap() {
            if let Err(why) = discord_bot.remove_ticket(&ticket).await {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": why.to_string()
                    })),
                )
            } else {
                let result = mongodb
                    .database(&CONFIG.mongo_database)
                    .collection::<Ticket>("tickets")
                    .delete_one(
                        doc! {
                            "_id": ObjectId::from_str(&ticket_id).expect("Couldn't transform OID")
                        },
                        None,
                    )
                    .await;

                if let Ok(DeleteResult {
                    deleted_count: 1, ..
                }) = result
                {
                    (StatusCode::OK, Json(json!({})))
                } else {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "error": result.unwrap_err().to_string()
                        })),
                    )
                }
            }
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Could not find ticket" })),
            )
        }
    }
}

async fn get_tickets(Extension(mongodb): Extension<Client>) -> impl IntoResponse {
    let data = mongodb
        .database(&CONFIG.mongo_database)
        .collection::<Ticket>("tickets")
        .find(None, None)
        .await;

    if let Ok(tickets) = data {
        let tickets = tickets
            .filter_map(|x| async move { x.ok() })
            .collect::<Vec<Ticket>>()
            .await;

        (StatusCode::OK, Json(json!({ "tickets": tickets })))
    } else {
        let why = data.unwrap_err().to_string();

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": why })),
        )
    }
}

async fn add_ticket(
    Json(ticket): Json<Ticket>,
    Extension(mongodb): Extension<Client>,
    Extension(discord_bot): Extension<DynDiscordBotAPI>,
) -> impl IntoResponse {
    let mut ticket_to_insert = ticket.with_id();
    
    log::debug!("Sending ticket to discord bot");
    
    let result = discord_bot.add_ticket(&ticket_to_insert).await;

    if let Err(why) = result {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": why.to_string()
            })),
        )
    } else {
        let channel_id = result.unwrap();
        
        ticket_to_insert.channel_id = Some(channel_id);
        
        log::debug!("Inserting ticket {ticket_to_insert:?}");
        let data = mongodb
            .database(&CONFIG.mongo_database)
            .collection::<Ticket>("tickets")
            .insert_one(&ticket_to_insert, None)
            .await;

        let value = serde_json::to_value(&ticket_to_insert).unwrap();

        (StatusCode::OK, Json(value))
    }

}
