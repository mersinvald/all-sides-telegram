use fantoccini::Client;
use select::document::Document;
use serde_json::json;
use webdriver::capabilities::Capabilities;

pub struct HtmlLoader {
    client: Client,
}

impl HtmlLoader {
    pub async fn try_new(wd_host: &str, wd_port: u16) -> Result<Self, anyhow::Error> {
        let mut caps = Capabilities::new();
        caps.insert(
            "moz:firefoxOptions".into(),
            json!({
                "args": ["-headless"]
            }),
        );
        let url = format!("http://{}:{}", wd_host, wd_port);
        let client = Client::with_capabilities(&url, caps).await?;
        Ok(Self { client })
    }

    pub async fn open(&mut self, url: &str) -> Result<Document, anyhow::Error> {
        self.client.goto(url).await?;
        let source = self.client.source().await?;
        Ok(Document::from(source.as_str()))
    }
}
