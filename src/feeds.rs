use super::Edgar;
use super::FeedOperations;
use super::error::{EdgarError, Result};
use super::options::FeedOptions;
use async_trait::async_trait;
use parsers::{
    atom::{AtomConfig, AtomDocument, AtomParser},
    rss::{RssConfig, RssDocument, RssParser},
};

/// Implements feed operations for the SEC EDGAR system.
///
/// This implementation provides methods to fetch and parse various SEC EDGAR feeds,
/// including current filings, company-specific filings, press releases, and various
/// specialized RSS feeds.
///
/// # Feed Types
///
/// The implementation supports two main types of feeds:
/// * Atom feeds (for current and company-specific filings)
/// * RSS feeds (for news, alerts, and specialized content)
///
/// # Examples
///
/// ```no_run
/// use your_crate::Edgar;
/// use your_crate::FeedOperations;
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("your-email@example.com");
///
///     // Fetch current filings
///     let current = edgar.current_feed(None).await?;
///
///     // Fetch company-specific filings
///     let company = edgar.company_feed("1018724", None).await?;
///
///     // Fetch press releases
///     let press = edgar.press_release_feed().await?;
///
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
        parser
            .parse(content)
            .map_err(|e| EdgarError::ParserError(e))
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
        parser
            .parse(content)
            .map_err(|e| EdgarError::ParserError(e))
    }

    /// Fetches various RSS feeds
    async fn get_rss_feed(&self, url: &str) -> Result<RssDocument> {
        let content = self.get(url).await?;
        self.rss_feed_from_string(&content)
    }

    /// Parses an RSS feed from a string
    fn rss_feed_from_string(&self, content: &str) -> Result<RssDocument> {
        let parser = RssParser::new(RssConfig::default());
        parser
            .parse(content)
            .map_err(|e| EdgarError::ParserError(e))
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

    /// Fetches the litigation feed
    async fn litigation_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/litigation/litreleases.xml")
            .await
    }

    /// Fetches the administrative proceedings feed
    async fn administrative_proceedings_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/litigation/admin.xml")
            .await
    }

    /// Fetches the trading suspensions feed
    async fn trading_suspensions_feed(&self) -> Result<RssDocument> {
        self.get_rss_feed("https://www.sec.gov/rss/litigation/suspensions.xml")
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
    use std::fs;

    const RSS_USGAAP_FIXTURE: &str = "fixtures/rss/usgaap.rss";
    const RSS_PRESSREL_FIXTURE: &str = "fixtures/rss/pressreleases.rss";
    const RSS_TESTIMONY_FIXTURE: &str = "fixtures/rss/testimony.rss";
    const ATOM_FIXTURE: &str = "fixtures/atom/atom.xml";
    const ATOM1_FIXTURE: &str = "fixtures/atom/atom1.xml";

    fn setup_edgar() -> Edgar {
        Edgar::new("test_agent example@example.com").unwrap()
    }

    #[test]
    fn test_parse_testimony_feed() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(RSS_TESTIMONY_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        assert_eq!(feed.channel.title, "Testimony");
        assert_eq!(feed.channel.language.as_deref().unwrap(), "en");
        assert!(feed.channel.description.contains("Congressional testimony"));
    }

    #[test]
    fn test_parse_feed_with_multiple_categories() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(ATOM1_FIXTURE).unwrap();
        let feed = edgar.current_feed_from_string(&content).unwrap();

        let categories: Vec<_> = feed
            .entries
            .iter()
            .filter_map(|e| e.category.as_ref())
            .map(|c| c.term.as_str())
            .collect();

        assert!(categories.contains(&"8-K"));
        assert!(categories.contains(&"425"));
        assert!(categories.contains(&"SC 13G"));
    }

    #[test]
    fn test_xml_namespaces() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(RSS_USGAAP_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        assert!(feed.channel.atom_link.is_some());
        let atom_link = feed.channel.atom_link.unwrap();
        assert_eq!(atom_link.rel.as_deref(), Some("self"));
        assert!(atom_link.href.contains("usgaap.rss.xml"));
    }

    #[test]
    fn test_feed_dates() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(RSS_PRESSREL_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        // Don't check lastBuildDate as it's not present in pressreleases.rss
        // Check other date-related fields
        let first_item = &feed.channel.items[0];
        assert!(first_item.pub_date.is_some());
        assert_eq!(
            first_item.pub_date.as_deref().unwrap(),
            "Fri, 24 Jan 2025 11:00:00 -0500"
        );

        // Test multiple items have dates
        assert!(
            feed.channel
                .items
                .iter()
                .all(|item| item.pub_date.is_some())
        );
    }

    #[test]
    fn test_enclosure_handling() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(RSS_USGAAP_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        let first_item = &feed.channel.items[0];
        let enclosure = first_item.enclosure.as_ref().unwrap();

        assert!(enclosure.url.contains("xbrl.zip"));
        assert!(enclosure.length.is_some());
        assert_eq!(enclosure.enclosure_type.as_deref(), Some("application/zip"));
    }

    #[test]
    fn test_special_characters() {
        let edgar = setup_edgar();
        let content = fs::read_to_string(RSS_PRESSREL_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        let items = &feed.channel.items;
        for item in items {
            // Test HTML entities are decoded
            assert!(!item.description.as_ref().unwrap().contains("&amp;"));
            assert!(!item.description.as_ref().unwrap().contains("&lt;"));
            assert!(!item.description.as_ref().unwrap().contains("&gt;"));
        }
    }

    #[test]
    fn test_empty_feed() {
        let edgar = setup_edgar();
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

    #[test]
    fn test_parse_rss_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let content = fs::read_to_string(RSS_USGAAP_FIXTURE).unwrap();
        let feed = edgar.rss_feed_from_string(&content).unwrap();

        assert_eq!(
            feed.channel.title,
            "Filings containing financial statements tagged using the US GAAP or IFRS taxonomies."
        );
        assert!(feed.channel.items.len() > 0);
    }

    #[test]
    fn test_parse_atom_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let content = fs::read_to_string(ATOM_FIXTURE).unwrap();
        let feed = edgar.current_feed_from_string(&content).unwrap();

        assert!(feed.entries.len() > 0);
        assert!(feed.company_info.is_some());
    }

    #[tokio::test]
    async fn test_current_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let feed = edgar.current_feed(None).await.unwrap();
        assert!(!feed.entries.is_empty());
    }

    #[tokio::test]
    async fn test_current_feed_with_options() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let params = FeedOptions::new(None)
            .with_param("count", "10")
            .with_param("type", "10-K");

        let feed = edgar.current_feed(Some(params)).await.unwrap();
        assert!(!feed.entries.is_empty());
    }

    #[tokio::test]
    async fn test_company_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let feed = edgar.company_feed("320193", None).await.unwrap();
        assert!(!feed.entries.is_empty());
    }

    #[tokio::test]
    async fn test_company_feed_with_options() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let params = FeedOptions::new(None)
            .with_param("count", "10")
            .with_param("type", "10-K");

        let feed = edgar.company_feed("320193", Some(params)).await.unwrap();
        assert!(!feed.entries.is_empty());
    }

    #[tokio::test]
    async fn test_press_release_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let feed = edgar.press_release_feed().await.unwrap();
        assert!(!feed.channel.items.is_empty());
    }

    #[tokio::test]
    async fn test_historical_xbrl_feed() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let feed = edgar.historical_xbrl_feed(2021, 1).await.unwrap();
        assert!(!feed.channel.items.is_empty());
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
        let edgar = setup_edgar();
        let result = edgar.rss_feed_from_string("invalid xml");
        assert!(result.is_err());
    }
}
