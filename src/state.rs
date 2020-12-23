use std::path::Path;

#[derive(Debug)]
pub struct State {
    stories: sled::Db,
}

impl State {
    pub fn try_new(stories_db_path: &Path) -> anyhow::Result<Self> {
        let db = sled::open(stories_db_path)?;
        Ok(State { stories: db })
    }

    pub fn is_published(&self, url: &str) -> anyhow::Result<bool> {
        Ok(self.stories.contains_key(url)?)
    }

    pub async fn set_published(&mut self, url: &str) -> anyhow::Result<()> {
        self.stories.insert(url, "true")?;
        self.stories.flush_async().await?;
        Ok(())
    }
}
