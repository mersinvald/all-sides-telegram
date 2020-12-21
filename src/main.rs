mod config;
mod loader;
mod scraper;
mod state;
mod tg_bot;

use config::Config;
use loader::HtmlLoader;
use scraper::{FromHTML, MainPage, Story};
use state::State;
use tg_bot::Bot;

use handlebars::Handlebars;
use itertools::Itertools;
use serde_json::json;
use std::time::Duration;

const ALL_SIDES_MAINPAGE: &str = "https://www.allsides.com/unbiased-balanced-news";

struct AllSidesTgImporter {
    cfg: Config,
    loader: HtmlLoader,
    bot: Bot,
    state: State,
    template: Handlebars<'static>,
}

impl AllSidesTgImporter {
    pub async fn try_new(cfg: Config) -> anyhow::Result<AllSidesTgImporter> {
        let loader = HtmlLoader::try_new(&cfg.webdriver_host, cfg.webdriver_port).await?;
        let bot = Bot::try_new(&cfg.telegram)?;
        let state = State::try_new(&cfg.story_db)?;
        let mut template = Handlebars::new();
        template
            .register_template_string("main", include_str!("../data/post-template.handlebars"))?;
        Ok(AllSidesTgImporter {
            cfg,
            loader,
            bot,
            state,
            template,
        })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let delay = Duration::from_secs(self.cfg.update_interval * 60);
        loop {
            if let Err(e) = self.tick().await {
                log::error!("{}", e);
                self.bot
                    .log_error(e)
                    .await
                    .map_err(|e| log::error!("failed to post log message to telegram: {}", e))
                    .ok();
            }
            tokio::time::delay_for(delay).await;
        }
    }

    async fn tick(&mut self) -> anyhow::Result<()> {
        let main_page = self.loader.open(ALL_SIDES_MAINPAGE).await?;
        let main_page = MainPage::from_html(&main_page)?;
        for teaser in main_page.teasers {
            if self.state.is_published(&teaser.url)? {
                continue;
            }

            let story = self.loader.open(&teaser.url).await?;
            let story = Story::from_html(&story)?;

            self.publish_story(&story, &teaser.url).await?;
        }
        Ok(())
    }

    async fn publish_story(&mut self, story: &Story<'_>, url: &str) -> anyhow::Result<()> {
        let formatted = self.format_story(story, url)?;
        self.bot.publish_message(formatted).await?;
        self.state.set_published(url).await
    }

    fn format_story(&self, story: &Story, url: &str) -> anyhow::Result<String> {
        let story_content = story.summary.iter().map(|p| p.telegram_html()).join("\n\n");

        let side_stories = story
            .articles
            .iter()
            .map(|article| {
                let article_content = article
                    .summary
                    .iter()
                    .map(|p| p.telegram_html())
                    .take_while(|para| !para.ends_with("..."))
                    .join("\n\n");

                json!({
                    "side_story_emoji": article.side.emoji(),
                    "side_story_title": article.title,
                    "side_story_url": article.url,
                    "side_story_source": article.source,
                    "side_story_content": article_content,
                })
            })
            .collect::<Vec<_>>();

        let side_stories = serde_json::Value::Array(side_stories);

        let data = json!({
            "story_title": story.title,
            "story_content": story_content,
            "story_url": url,
            "story_date": format!("{}", story.datetime.date().format("%Y-%m-%d")),
            "side_stories": side_stories,
        });

        Ok(self.template.render("main", &data)?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let config = envy::prefixed("ASTG_").from_env::<Config>()?;
    let bot = AllSidesTgImporter::try_new(config).await?;
    bot.run().await?;
    Ok(())
}
