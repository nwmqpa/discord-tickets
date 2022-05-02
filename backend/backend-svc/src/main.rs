use std::net::SocketAddr;

use axum::Router;
use mongodb::{Client, options::ClientOptions};
use once_cell::sync::Lazy;
use serde::Deserialize;
use tracing::log;


static CONFIG: Lazy<Config> = Lazy::new(|| {
    match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(error) => panic!("{:#?}", error)
    }
});

#[derive(Deserialize)]
struct Config {
    mongo_host: String
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut client_options = ClientOptions::parse(&CONFIG.mongo_host).await?;

    // Manually set an option.
    client_options.app_name = Some("Backend Service".to_string());
    
    // Get a handle to the deployment.
    let _client = Client::with_options(client_options)?;
    

    // build our application with a route
    let app = Router::new();

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
