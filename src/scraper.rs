use anyhow::{anyhow, bail};
use chrono::{DateTime, FixedOffset};
use select::document::Document;
use select::node;
use select::predicate::{Attr, Class, Name};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct MainPage {
    pub teasers: Vec<Teaser>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    CenterLeft,
    Center,
    CenterRight,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teaser {
    pub title: String,
    pub url: String,
    pub img_url: String,
}

#[derive(Debug, Clone)]
pub struct Paragraph<'a>(node::Node<'a>);

#[derive(Debug, Clone)]
pub struct Story<'a> {
    pub title: String,
    pub summary: Vec<Paragraph<'a>>,
    pub articles: Vec<Article<'a>>,
    pub datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct Article<'a> {
    pub side: Side,
    pub source: String,
    pub title: String,
    pub summary: Vec<Paragraph<'a>>,
    pub url: String,
}

impl FromStr for Side {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = match s {
            "Left" => Self::Left,
            "Lean Left" | "Center Left" => Self::CenterLeft,
            "Center" => Self::Center,
            "Lean Right" | "Center Right" => Self::CenterRight,
            "Right" => Self::Right,
            _ => bail!("unexpected political affiliation shorthand: {}", s),
        };

        Ok(x)
    }
}

impl Side {
    pub fn emoji(&self) -> &'static str {
        match *self {
            Side::Left => "🟦",
            Side::CenterLeft => "🔵",
            Side::Center => "🟣",
            Side::CenterRight => "🔴",
            Side::Right => "🟥",
        }
    }
}

impl Paragraph<'_> {
    pub fn text(&self) -> String {
        self.0.text()
    }

    pub fn telegram_html(&self) -> String {
        self.0
            .children()
            .map(|n| {
                Some(n)
                    .filter(|n| n.is(Name("a")))
                    .and_then(|a| {
                        Some(format!(
                            "<a href=\"{url}\">{text}</a>",
                            url = a.attr("href")?,
                            text = a.text()
                        ))
                    })
                    .unwrap_or_else(|| n.text())
            })
            .collect::<String>()
    }
}

pub trait FromHTML<'a>: 'a + Sized {
    fn from_html(html: &'a Document) -> anyhow::Result<Self>;
}

impl FromHTML<'_> for MainPage {
    fn from_html(html: &Document) -> anyhow::Result<Self> {
        let mut teasers: Vec<_> = html.find(Class("view-story-id-single-story")).map(|block| {
            let url = block.find(Name("a")).next()
                .and_then(|node| node.attr("href"))
                .ok_or_else(|| anyhow!("cannot query story url (class=view-story-id-single-story > a.href)"))?;

            let title = block.find(Class("story-title")).next()
                .ok_or_else(|| anyhow!("cannot query story url (class=view-story-id-single-story > class=story-title)"))?
                .text();

            let img_url = block.find(Class("story-id-image")).next()
                .and_then(|node| node.find(Name("img")).next())
                .and_then(|node| node.attr("src"))
                .ok_or_else(|| anyhow!("cannot query story url (class=view-story-id-single-story > 0 > img.src)"))?;

            Ok(Teaser {
                title,
                url: normalize_allsides_url(url),
                img_url: img_url.to_owned(),
            })
        }).collect::<Result<_, anyhow::Error>>()?;

        // Reverse posts for them to be in chronological order
        teasers.reverse();

        if teasers.is_empty() {
            bail!("the main page contains no stories: parsing is broken");
        }

        Ok(MainPage { teasers })
    }
}

impl<'a> FromHTML<'a> for Story<'a> {
    fn from_html(html: &'a Document) -> anyhow::Result<Self> {
        let story = html
            .find(Attr("id", "content"))
            .next()
            .ok_or_else(|| anyhow!("cannot query story body (id=content)"))?;

        let title = story
            .find(Class("taxonomy-heading"))
            .next()
            .ok_or_else(|| anyhow!("cannot query story heading (class=taxonomy-heading)"))?
            .text()
            .trim()
            .into();

        let datetime = story
            .find(Class("date-display-single"))
            .next()
            .and_then(|node| node.attr("content"))
            .ok_or_else(|| {
                anyhow!(
                    "cannot query story publishing date (class=date-display-single, attr=content)"
                )
            })?;
        let datetime = chrono::DateTime::parse_from_rfc3339(datetime)
            .map_err(|err| anyhow!("unexpected date-time format (not rfc3339): {}", err))?;

        let description = html
            .find(Class("story-id-page-description"))
            .next()
            .ok_or_else(|| {
                anyhow!("cannot query story summary (class=story-id-page-description)")
            })?;
        let paragraphs: Vec<_> = description
            .children()
            .filter(|node| node.is(Name("p")))
            .map(Paragraph)
            .collect();
        if paragraphs.is_empty() {
            bail!("summary contains no paragraphs");
        }

        let articles = story
            .find(Class("feature-thumbs-wrapper"))
            .next()
            .ok_or_else(|| {
                anyhow!("cannot query linked stories container (class=feature-thumbs-wrapper)")
            })?;

        let articles = articles.find(Class("feature-thumbs"));
        let articles: Vec<_> = articles.map(|node| {
            let title = node.find(Class("news-title")).next()
                .and_then(|node| node.find(Name("a")).next())
                .map(|node| node.text())
                .ok_or_else(|| anyhow!("cannot query linked article title (class=news-title > a)"))?
                .trim().into();

            let url = node.find(Class("read-more-story")).next()
                .and_then(|node| node.find(Name("a")).next())
                .and_then(|node| node.attr("href"))
                .ok_or_else(|| anyhow!("cannot query linked article origin url (class=read-more-story > a.href)"))?
                .to_owned();

            let source = node.find(Class("news-source")).next()
                .ok_or_else(|| anyhow!("cannot query news article source (class=news-source)"))?
                .text();

            let bias = node.find(Class("bias-image")).next()
                .and_then(|node| node.first_child())
                .filter(|node| node.is(Name("img")))
                .and_then(|node| node.attr("title"))
                .ok_or_else(|| anyhow!("cannot query news source political bias (class=bias-image > img.title)"))?;

            if !bias.contains(':') || bias.is_empty() {
                bail!("unexpected bias format: expected '<any>: <affiliation>'");
            }

            let side_shorthand = bias.split(':').last()
                .ok_or_else(|| anyhow!("bug: unexpected bias format (pre-check passed, parsing failed)"))?
                .trim();

            let side = Side::from_str(side_shorthand)?;

            let description = node.find(Class("news-body")).next()
                .ok_or_else(|| anyhow!("cannot query article summary (class=news-body)"))?;
            let paragraphs: Vec<_> = description.children()
                .filter(|node| node.is(Name("p")))
                .map(Paragraph)
                .collect();

            Ok(Article {
                side,
                source,
                title,
                summary: paragraphs,
                url,
            })
        }).collect::<Result<_, _>>()?;

        Ok(Story {
            title,
            summary: paragraphs,
            articles,
            datetime,
        })
    }
}

fn normalize_allsides_url(relative_url: &str) -> String {
    format!("https://www.allsides.com{}", relative_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_main_page() -> anyhow::Result<()> {
        let html = Document::from(include_str!("../data/allsides-main-page.html"));
        let parsed = MainPage::from_html(&html)?;
        assert_eq!(&parsed.teasers, &[
            Teaser {
                title: "McConnell Recognizes Biden as President-Elect".into(),
                url: "https://www.allsides.com/story/mcconnell-recognizes-biden-president-elect".into(),
                img_url: "https://www.allsides.com/sites/default/files/styles/feature_image_300x200/public/mreportmcconnell_1_1.jpg?itok=fYKpR74i".into(),
            },
            Teaser {
                title: "Russian Hackers Suspected in Broad Attack on US Government, Businesses".into(),
                url: "https://www.allsides.com/story/russian-hackers-suspected-broad-attack-us-government-businesses".into(),
                img_url: "https://www.allsides.com/sites/default/files/styles/feature_image_300x200/public/594d77f2-7693-4a33-8d79-5c007f8ffb7d%40news.ap_.org_.jpg?itok=g5Jm68U3".into(),
            },
            Teaser {
                title: "NY Gov. Cuomo Accused of Sexual Harrassment; Less Coverage from Left-Rated Outlets".into(),
                url: "https://www.allsides.com/story/ny-gov-cuomo-accused-sexual-harrassment-less-coverage-left-rated-outlets".into(),
                img_url: "https://www.allsides.com/sites/default/files/styles/feature_image_300x200/public/cuo.png?itok=NeTuIYlx".into(),
            },
        ]);
        Ok(())
    }

    #[test]
    fn parse_story() -> anyhow::Result<()> {
        let html = Document::from(include_str!("../data/allsides-story.html"));
        let parsed = Story::from_html(&html)?;
        assert_eq!(
            parsed.title,
            "NY Gov. Cuomo Accused of Sexual Harrassment; Less Coverage from Left-Rated Outlets"
        );
        assert_eq!(
            parsed.summary[0].text(),
            r#"A former aide accused New York Gov. Andrew Cuomo (D) of sexually harassing her while she worked for him between 2015 and 2018. Lindsey Boylan, a current candidate for Manhattan borough president, said "Yes, @NYGovCuomo sexually harassed me for years. Many saw it, and watched" in a tweet Sunday morning. The governor's office responded by saying "There is simply no truth to these claims." "#
        );
        assert_eq!(
            parsed.summary[1].text(),
            r#"Right-rated outlets reported the story more prominently than left- and center-rated outlets. Coverage from the right focused on the fact that many left-rated news sources, including CNN where Cuomo's brother Chris works as an anchor, had not covered the story, framing the sources as hypocritical and protective of Democrats. Some coverage from left- and center-rated outlets concentrated on Boylan's claims; others highlighted the governor's denial and other doubts about the allegations."#
        );
        assert_eq!(parsed.datetime.timestamp(), 1608079500);

        let article_0 = &parsed.articles[0];
        assert_eq!(
            article_0.title,
            "The sexual harassment allegation against Gov. Andrew Cuomo, explained"
        );
        assert_eq!(
            article_0.url,
            "https://www.vox.com/22174452/andrew-cuomo-lindsey-boylan-sexual-harassment"
        );
        assert_eq!(article_0.source, "Vox");
        assert_eq!(article_0.side, Side::Left);
        assert_eq!(
            article_0.summary[0].text(),
            r#"“Yes, @NYGovCuomo sexually harassed me for years. Many saw it, and watched.”"#
        );
        assert_eq!(
            article_0.summary[1].text(),
            r#"So said Lindsey Boylan, a candidate for Manhattan borough president and former adviser to New York Gov. Andrew Cuomo, in a tweet thread Sunday morning."#
        );
        assert_eq!(
            article_0.summary[2].text(),
            r#"Boylan, who appears to have worked for the governor’s office from 2015 to 2018, provided few specifics in the thread and stated that she has “no interest in talking to journalists” (Boylan also has not responded to Vox’s request for comment). But she did say, “I could never anticipate what to expect: would..."#
        );

        let article_1 = &parsed.articles[1];
        assert_eq!(
            article_1.title,
            "Mainstream media ignores sexual harassment allegations against Gov. Andrew Cuomo"
        );
        assert_eq!(article_1.url, "https://www.foxnews.com/media/mainstream-media-ignores-sexual-harassment-allegations-against-gov-andrew-cuomo");
        assert_eq!(article_1.source, "Fox News (Online News)");
        assert_eq!(article_1.side, Side::CenterRight);
        assert_eq!(
            article_1.summary[0].text(),
            r#"New York City politico Lindsey Boylan alleged on Twitter Sunday that New York Gov. Andrew Cuomo "sexually harassed me for years," but anyone who relies on CNN, MSNBC, ABC, NBC or CBS for news would have no idea."#
        );
        assert_eq!(
            article_1.summary[1].text(),
            r#"Boylan, who describes herself as a progressive on her Twitter account, worked for the governor's administration from 2015 to 2018, according to her LinkedIn profile."#
        );
        assert_eq!(
            article_1.summary[2].text(),
            r#""Yes, @NYGovCuomo sexually harassed me for years," Boylan tweeted. "Many saw it, and watched. I could never anticipate what to expect: would I be grilled on my..."#
        );

        let article_2 = &parsed.articles[2];
        assert_eq!(
            article_2.title,
            "MeToo double standard: Evidence required when accused is a Democrat"
        );
        assert_eq!(article_2.url, "https://nypost.com/2020/12/14/metoo-double-standard-evidence-required-when-accused-is-a-democrat/");
        assert_eq!(article_2.source, "New York Post (Opinion)");
        assert_eq!(article_2.side, Side::Right);
        assert_eq!(
            article_2.summary[0].text(),
            r#"Gov. Andrew Cuomo is lucky he’s a Democrat — otherwise Lindsey Boylan’s charge that he “sexually harassed” her might lead to political challenges and media shame."#
        );
        assert_eq!(
            article_2.summary[1].text(),
            r#"“Yes, @NYGovCuomo sexually harassed me for years. Many saw it, and watched,” the former Cuomo aide, now running for Manhattan beep, tweeted."#
        );
        assert_eq!(
            article_2.summary[2].text(),
            r#"It’s a serious charge, one Cuomo flatly denies. The details? Boylan won’t say. She cites no specific allegations and provides no evidence. She won’t even respond to requests for further comment. “I have no interest in talking to journalists,” she declares. “I..."#
        );

        Ok(())
    }

    #[test]
    fn paragraph_to_text() -> anyhow::Result<()> {
        let html = Document::from(include_str!("../data/allsides-story.html"));
        let parsed = Story::from_html(&html)?;
        assert_eq!(
            parsed.summary[0].text(),
            r#"A former aide accused New York Gov. Andrew Cuomo (D) of sexually harassing her while she worked for him between 2015 and 2018. Lindsey Boylan, a current candidate for Manhattan borough president, said "Yes, @NYGovCuomo sexually harassed me for years. Many saw it, and watched" in a tweet Sunday morning. The governor's office responded by saying "There is simply no truth to these claims." "#
        );
        Ok(())
    }

    #[test]
    fn paragraph_to_tg_html() -> anyhow::Result<()> {
        let html = Document::from(include_str!("../data/allsides-story.html"));
        let parsed = Story::from_html(&html)?;
        assert_eq!(
            parsed.summary[0].telegram_html(),
            r#"A former aide accused New York Gov. Andrew Cuomo (D) of sexually harassing her while she worked for him between 2015 and 2018. Lindsey Boylan, a current candidate for Manhattan borough president, said "Yes, @NYGovCuomo sexually harassed me for years. Many saw it, and watched" <a href="https://twitter.com/LindseyBoylan/status/1338125549756182529">in a tweet</a> Sunday morning. The governor's office responded by saying "There is simply no truth to these claims." "#
        );
        Ok(())
    }
}
