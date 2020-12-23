use crate::config::TelegramOptions;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

pub struct Bot {
    bot: teloxide::Bot,
    channel_id: String,
    admin_id: String,
}

impl Bot {
    pub fn try_new(opts: &TelegramOptions) -> anyhow::Result<Self> {
        let bot = teloxide::Bot::builder()
            .token(&opts.secret)
            .parse_mode(ParseMode::HTML)
            .build();
        Ok(Bot {
            bot,
            channel_id: opts.channel.clone(),
            admin_id: opts.admin.clone(),
        })
    }

    pub async fn log_error(&self, err: impl std::fmt::Display) -> anyhow::Result<()> {
        self.bot
            .send_message(
                ChatId::ChannelUsername(self.admin_id.clone()),
                err.to_string(),
            )
            .send()
            .await?;
        Ok(())
    }

    pub async fn publish_message(&self, msg: impl std::fmt::Display) -> anyhow::Result<()> {
        self.bot
            .send_message(
                ChatId::ChannelUsername(self.channel_id.clone()),
                msg.to_string(),
            )
            .send()
            .await?;
        Ok(())
    }
}
