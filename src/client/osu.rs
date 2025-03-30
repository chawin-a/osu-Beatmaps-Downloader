use super::{Beatmapset, SearchClient};
use crate::utils::empty_string_as_none;
use async_trait::async_trait;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

pub struct OsuClient {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BeatmapsetSearchResult {
    pub beatmapsets: Vec<Beatmapset>,
    #[serde(deserialize_with = "empty_string_as_none")]
    pub cursor_string: Option<String>,
}

impl OsuClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn search(&self, cursor_string: Option<String>) -> Result<BeatmapsetSearchResult> {
        let url = format!(
            "https://osu.ppy.sh/beatmapsets/search?m=0&s=ranked&nsfw=true&cursor_string={}&sort=ranked_desc",
            if let Some(cursor_string) = cursor_string {
                cursor_string
            } else {
                "".to_owned()
            }
        );
        let res = self.client.get(url).send().await?;
        if res.status().is_success() {
            let result = res
                .json::<BeatmapsetSearchResult>()
                .await
                .map_err(|e| eyre!("can't map to beatmapset search result struct: {}", e))?;
            Ok(result)
        } else {
            Err(eyre!("Request failed with status: {}", res.status()))
        }
    }
}

#[async_trait]
impl SearchClient for OsuClient {
    async fn fetch_new_songs(&self, num: u32) -> Result<Vec<Beatmapset>> {
        let mut cursor = None;
        let mut songs = Vec::new();
        let n = num / 50;
        for _ in 0..n {
            let res = self.search(cursor).await?;
            songs.extend(res.beatmapsets);
            if res.cursor_string.is_none() {
                break;
            }
            cursor = res.cursor_string;
        }
        Ok(songs)
    }
}
