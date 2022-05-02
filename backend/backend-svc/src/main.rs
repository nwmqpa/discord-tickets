use std::{net::SocketAddr, str::FromStr};

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Extension, Json, Router,
};
use futures::StreamExt;
use mongodb::{bson::{oid::ObjectId, doc}, options::ClientOptions, Client};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::log;

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
struct Ticket {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    title: String,
    content: String,
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

    // build our application with a route
    let app = Router::new()
        .route("/tickets", get(get_tickets))
        .route("/tickets", post(add_ticket))
        .route("/tickets/:mongo_id", delete(delete_ticket))
        .layer(Extension(client));

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
) -> impl IntoResponse {
    let data = mongodb
        .database(&CONFIG.mongo_database)
        .collection::<Ticket>("tickets")
        .delete_one(
            doc! {
                "_id": ObjectId::from_str(&ticket_id).expect("Couldn't transform OID")
            },
            None,
        )
        .await;

    if let Ok(data) = data {

        log::debug!("{data:?}");

        (StatusCode::OK, Json(json!({})))
    } else {
        let why = data.unwrap_err().to_string();
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": why })),
        )
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
) -> impl IntoResponse {
    let ticket_to_insert = ticket.with_id();

    let data = mongodb
        .database(&CONFIG.mongo_database)
        .collection::<Ticket>("tickets")
        .insert_one(&ticket_to_insert, None)
        .await;

    log::debug!("Inserting ticket {data:?}");

    (StatusCode::OK, Json(ticket_to_insert))
}
