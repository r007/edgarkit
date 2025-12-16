//! Atom and RSS feeds for EDGAR filings and SEC news.
//!
//! The SEC exposes two main feed formats:
//! - **Atom** feeds for EDGAR browsing endpoints such as “current filings” and
//!   company-specific listings.
//! - **RSS** feeds for press releases, speeches, statements, and various EDGAR
//!   dissemination streams.
//!
//! This module implements `FeedOperations` for [`Edgar`]. Network methods return parsed
//! `AtomDocument`/`RssDocument` values, and companion `*_from_string` helpers make it easy
//! to test parsing against fixtures or to integrate with custom download logic.

use super::Edgar;
use super::FeedOperations;
use super::error::{EdgarError, Result};
use super::options::FeedOptions;
use crate::parsing::{
    atom::{AtomConfig, AtomDocument, AtomParser},
    rss::{RssConfig, RssDocument, RssParser},
};
use async_trait::async_trait;

/// Feed operations for SEC EDGAR.
///
/// # Examples
///
/// ```ignore
/// use edgarkit::{Edgar, FeedOperations, FeedOptions};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("MyApp contact@example.com")?;
///
///     // Atom: current filings (optionally parameterized).
///     let current = edgar.current_feed(None).await?;
///     let opts = FeedOptions::new(None).with_param("count", "25");
///     let current_limited = edgar.current_feed(Some(opts)).await?;
///
///     // Atom: company-specific feed.
///     let company = edgar.company_feed("1018724", None).await?;
///
///     // RSS: SEC news.
///     let press = edgar.press_release_feed().await?;
///
///     println!("current={}, current_limited={}, company={}, press={}", current.entries.len(), current_limited.entries.len(), company.entries.len(), press.channel.items.len());
///     Ok(())
/// }
/// ```
///
/// # Notes
///
/// * Historical XBRL feeds are only available from 2005 onwards
/// * All feed operations require proper initialization of the Edgar client with a valid user agent
/// * Some feeds might require proper rate limiting to comply with SEC.gov's fair access rules
#[async_trait]
impl FeedOperations for Edgar {
    /// Fetches the current feed
    async fn current_feed(&self, opts: Option<FeedOptions>) -> Result<AtomDocument> {
        let feed_opts = FeedOptions::new(opts);
        let query = serde_urlencoded::to_string(feed_opts.params())
            .map_err(|e| EdgarError::InvalidResponse(e.to_string()))?;

        let url = format!(
            "https://www.sec.gov/cgi-bin/browse-edgar?action=getcurrent&{}",
            query
        );

        let content = self.get(&url).await?;
        self.current_feed_from_string(&content)
    }

    /// Parses the current feed from a string
    fn current_feed_from_string(&self, content: &str) -> Result<AtomDocument> {
        let parser = AtomParser::new(AtomConfig::default());
        parser.parse(content)
    }

    /// Fetches the company feed for a given CIK
    async fn company_feed(&self, cik: &str, opts: Option<FeedOptions>) -> Result<AtomDocument> {
        let feed_opts = FeedOptions::new(opts).with_param("CIK", cik);
        let query = serde_urlencoded::to_string(feed_opts.params())
            .map_err(|e| EdgarError::InvalidResponse(e.to_string()))?;

        let url = format!(
            "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&{}",
            query
        );

        let content = self.get(&url).await?;
        self.company_feed_from_string(&content)
    }

    /// Parses the company feed from a string
    fn company_feed_from_string(&self, content: &str) -> Result<AtomDocument> {
        let parser = AtomParser::new(AtomConfig::default());
        parser.parse(content)
    }

    /// Fetches various RSS feeds
    async fn get_rss_feed(&self, url: &str) -> Result<RssDocument> {
        let content = self.get(url).await?;
        self.rss_feed_from_string(&content)
    }

    /// Parses an RSS feed from a string
    fn rss_feed_from_string(&self, content: &str) -> Result<RssDocument> {
        let parser = RssParser::new(RssConfig::default());
        parser.parse(content)
    }

    /// Fetches the press release feed
    async fn press_release_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/news/pressreleases.rss")
            .await
    }

    /// Fetches the speeches and statements feed
    async fn speeches_and_statements_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/news/speeches-statements.rss")
            .await
    }

    /// Fetches the speeches feed
    async fn speeches_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/news/speeches.rss")
            .await
    }

    /// Fetches the statements feed
    async fn statements_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/news/statements.rss")
            .await
    }

    /// Fetches the testimony feed
    async fn testimony_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/news/testimony.rss")
            .await
    }

    /// Fetches the administrative proceedings feed
    async fn administrative_proceedings_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/litigation/admin.xml")
            .await
    }

    /// Fetches the division of corporation finance feed
    async fn division_of_corporation_finance_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/divisions/corpfin/cfnew.xml")
            .await
    }

    /// Fetches the division of investment management feed
    async fn division_of_investment_management_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/divisions/investment/imnews.xml")
            .await
    }

    /// Fetches the investor alerts feed
    async fn investor_alerts_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/investor/alerts")
            .await
    }

    /// Fetches the filings feed
    async fn filings_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/Archives/edgar/usgaap.rss.xml")
            .await
    }

    /// Fetches the mutual funds feed
    async fn mutual_funds_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/Archives/edgar/xbrl-rr.rss.xml")
            .await
    }

    /// Fetches the XBRL feed
    async fn xbrl_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/Archives/edgar/xbrlrss.all.xml")
            .await
    }

    /// Fetches the inline XBRL feed
    async fn inline_xbrl_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/Archives/edgar/xbrl-inline.rss.xml")
            .await
    }

    /// Fetches the historical XBRL feed
    async fn historical_xbrl_feed(&self, year: i32, month: i32) -> Result<RssDocument> {
        if year < 2005 {
            return Err(EdgarError::InvalidXBRLYear);
        }
        if month < 1 || month > 12 {
            return Err(EdgarError::InvalidMonth);
        }

        let url = format!(
            "https://www.sec.gov/Archives/edgar/monthly/xbrlrss-{}-{:02}.xml",
            year, month
        );
        self.get_rss_feed(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_empty_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let empty_rss = r#"<?xml version="1.0"?>
            <rss version="2.0">
                <channel>
                    <title>Empty Feed</title>
                    <link>http://example.com</link>
                    <description>Empty feed for testing</description>
                </channel>
            </rss>"#;

        let feed = edgar.rss_feed_from_string(empty_rss).unwrap();
        assert!(feed.channel.items.is_empty());
        assert_eq!(feed.channel.title, "Empty Feed");
        assert_eq!(feed.channel.description, "Empty feed for testing");
    }

    #[tokio::test]
    async fn test_historical_xbrl_feed_invalid_year() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.historical_xbrl_feed(2004, 1).await;
        assert!(matches!(result, Err(EdgarError::InvalidXBRLYear)));
    }

    #[tokio::test]
    async fn test_historical_xbrl_feed_invalid_month() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.historical_xbrl_feed(2005, 13).await;
        assert!(matches!(result, Err(EdgarError::InvalidMonth)));
    }

    #[test]
    fn test_invalid_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.rss_feed_from_string("invalid xml");
        assert!(result.is_err());
    }
}
