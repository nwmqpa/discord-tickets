use std::sync::Arc;

use async_trait::async_trait;
use futures::channel::oneshot::channel;
use reqwest::StatusCode;
use serde_json::Value;
use tracing::instrument;

use crate::Ticket;

pub type DynDiscordBotAPI = Arc<dyn DiscordBotAPI + Send + Sync>;

#[async_trait]
pub trait DiscordBotAPI {
    async fn add_ticket(&self, ticket: &Ticket) -> anyhow::Result<u64>;
    async fn remove_ticket(&self, ticket: &Ticket) -> anyhow::Result<()>;
}

#[derive(Default, Clone)]
pub struct ExternalDiscordBotAPI {
    client: reqwest::Client,
    url: String,
}

impl ExternalDiscordBotAPI {
    pub fn with_url<S: AsRef<str>>(url: S) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            ..Default::default()
        }
    }
}

#[async_trait]
impl DiscordBotAPI for ExternalDiscordBotAPI {

    #[instrument(skip(self))]
    async fn add_ticket(&self, ticket: &Ticket) -> anyhow::Result<u64> {
        let url = format!("{}/ticket", self.url);

        let result = self.client.post(url).json(&ticket).send().await?;

        anyhow::ensure!(result.status() == StatusCode::OK, "Adding ticket failed");

        let data: Value = result.json().await?;

        let channel_id = data.get("channel_id").and_then(|v| v.as_u64());

        anyhow::ensure!(channel_id.is_some(), "No channel id found or not u64");
        
        Ok(channel_id.unwrap())
    }

    #[instrument(skip(self))]
    async fn remove_ticket(&self, ticket: &Ticket) -> anyhow::Result<()> {
        let url = format!("{}/ticket", self.url);

        let result = self.client.delete(url).json(&ticket).send().await?;

        anyhow::ensure!(result.status() == StatusCode::OK, "Deleting ticket failed");

        Ok(())
    }
}
