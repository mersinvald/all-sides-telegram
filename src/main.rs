mod config;
mod loader;
mod scraper;
mod state;
mod tg_bot;

use self::config::Config;
use self::loader::HtmlLoader;
use self::scraper::{FromHTML, MainPage, Story};
use self::state::State;
use self::tg_bot::Bot;

use handlebars::Handlebars;
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
        let bot = Bot::try_new(
            &cfg.telegram_secret,
            &cfg.telegram_channel,
            &cfg.telegram_admin,
        )?;
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
        loop {
            if let Err(e) = self.tick().await {
                log::error!("{}", e);
                self.bot
                    .log_error(e)
                    .await
                    .expect("failed to log error into admin telegram chat");
            }
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
            let formatted = self.format_post_html(&story, &teaser.url)?;
            self.bot.publish_message(formatted).await?;
            self.state.set_published(&teaser.url).await?;
        }
        tokio::time::delay_for(Duration::from_secs(self.cfg.update_interval * 60)).await;
        Ok(())
    }

    fn format_post_html(&self, story: &Story, url: &str) -> anyhow::Result<String> {
        let story_content = story
            .summary
            .iter()
            .map(|p| p.text())
            .collect::<Vec<String>>()
            .join("\n\n");

        let side_stories = story
            .articles
            .iter()
            .map(|article| {
                let article_content = article
                    .summary
                    .iter()
                    .map(|p| p.text())
                    .filter(|para| !para.ends_with("..."))
                    .collect::<Vec<String>>()
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

        let side_stories = serde_json::to_value(side_stories)?;

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
    let config = envy::prefixed("ASTG_").from_env::<Config>()?;
    let bot = AllSidesTgImporter::try_new(config).await?;
    bot.run().await?;
    Ok(())
}
