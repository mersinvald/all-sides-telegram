use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

pub struct Bot {
    bot: teloxide::Bot,
    channel_id: String,
    admin_id: String,
}

impl Bot {
    pub fn try_new(token: &str, channel_id: &str, admin_id: &str) -> anyhow::Result<Self> {
        let bot = teloxide::Bot::builder()
            .token(token)
            .parse_mode(ParseMode::HTML)
            .build();
        Ok(Bot {
            bot,
            channel_id: channel_id.into(),
            admin_id: admin_id.into(),
        })
    }

    pub async fn log_error(&self, err: impl std::fmt::Display) -> anyhow::Result<()> {
        self.bot
            .send_message(
                ChatId::ChannelUsername(self.admin_id.clone()),
                format!("{}", err),
            )
            .send()
            .await?;
        Ok(())
    }

    pub async fn publish_message(&self, msg: impl std::fmt::Display) -> anyhow::Result<()> {
        self.bot
            .send_message(
                ChatId::ChannelUsername(self.channel_id.clone()),
                format!("{}", msg),
            )
            .send()
            .await?;
        Ok(())
    }
}
