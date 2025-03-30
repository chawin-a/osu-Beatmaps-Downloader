use super::{Beatmapset, SearchClient};
use async_trait::async_trait;

#[async_trait]
impl SearchClient for rosu_v2::Osu {
    async fn fetch_new_songs(&self, num: u32) -> eyre::Result<Vec<super::Beatmapset>> {
        let mut songs = Vec::new();
        let n = num / 50;
        let mut result = self
            .beatmapset_search()
            .mode(rosu_v2::prelude::GameMode::Osu)
            .status(Some(rosu_v2::prelude::RankStatus::Ranked))
            .await?;
        for _ in 0..n {
            for beatmap in result.mapsets.iter() {
                songs.push(Beatmapset {
                    id: beatmap.mapset_id,
                    title: beatmap.title.clone(),
                })
            }
            if !result.has_more() {
                break;
            }
            if let Some(res) = result.get_next(&self).await {
                result = res?;
            } else {
                return Err(eyre::eyre!("failed to get next page"));
            }
        }
        Ok(songs)
    }
}
