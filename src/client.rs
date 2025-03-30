pub mod nerinyan;
pub mod osu;
pub mod osu_api;

use async_trait::async_trait;
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Beatmapset {
    pub id: u32,
    pub title: String,
}
#[async_trait]
pub trait SearchClient: Send + Sync {
    async fn fetch_new_songs(&self, num: u32) -> Result<Vec<Beatmapset>>;
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[tokio::test]
    async fn test_nerinyan_search() {
        let client = nerinyan::NerinyanClient::new();
        assert_eq!(
            client
                .fetch_new_songs(100)
                .await
                .expect("failed to search song")
                .len(),
            100
        );
    }

    #[tokio::test]
    async fn test_osu_search() {
        let client = osu::OsuClient::new();
        assert_eq!(
            client
                .fetch_new_songs(100)
                .await
                .expect("failed to search song")
                .len(),
            100
        );
    }

    #[tokio::test]
    async fn test_osu_api_search() -> Result<()> {
        let config = crate::settings::read_config_from_yaml("config.yaml").unwrap();
        let client = rosu_v2::Osu::new(config.client_id, config.client_secret).await?;
        assert_eq!(
            client
                .fetch_new_songs(100)
                .await
                .expect("failed to search song")
                .len(),
            100
        );
        Ok(())
    }
}
