use super::{Beatmapset, SearchClient};
use async_trait::async_trait;
use eyre::{eyre, Ok, Result};

pub struct NerinyanClient {
    client: reqwest::Client,
}

impl NerinyanClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn search(&self, page: u32) -> Result<Vec<Beatmapset>> {
        let url = format!(
            "https://api.nerinyan.moe/search?m=0&s=ranked&nsfw=true&sort=ranked_desc&p={}&ps=50",
            page
        );
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("Mozilla/5.0"),
        );

        let res = self.client.get(url).headers(headers).send().await?;
        if res.status().is_success() {
            let result = res
                .json::<Vec<Beatmapset>>()
                .await
                .map_err(|e| eyre!("can't map to beatmapset search result struct: {}", e))?;
            Ok(result)
        } else {
            Err(eyre!("Request failed with status: {}", res.status()))
        }
    }
}

#[async_trait]
impl SearchClient for NerinyanClient {
    async fn fetch_new_songs(&self, num: u32) -> Result<Vec<Beatmapset>> {
        let n = num / 50;
        let mut res = Vec::new();
        for i in 0..n {
            let r = self.search(i).await?;
            res.extend(r);
        }
        Ok(res)
    }
}
