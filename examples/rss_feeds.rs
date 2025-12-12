//! RSS and Atom feeds example
//!
//! This example demonstrates how to access SEC RSS and Atom feeds:
//! - Current filings feed
//! - Company-specific feeds
//! - Press releases and news feeds
//! - Specialized SEC feeds
//!
//! Run with: `cargo run --example rss_feeds --all-features`

use edgarkit::{Edgar, FeedOperations, FeedOptions};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let edgar = Edgar::new("EdgarKit Example user@example.com")?;

    println!("=== EdgarKit RSS/Atom Feeds Example ===\n");

    // Example 1: Get current filings feed
    println!("1. Fetching current filings feed...");
    let current_feed = edgar.current_feed(None).await?;
    println!("✓ Feed Title: {}", current_feed.title);
    println!("✓ Last Updated: {}", current_feed.updated);
    println!("✓ Total Entries: {}", current_feed.entries.len());

    if let Some(first_entry) = current_feed.entries.first() {
        println!("\n   Most Recent Filing:");
        println!("   - Title: {}", first_entry.title);
    }

    // Example 2: Get company-specific feed with options
    println!("\n2. Fetching company-specific feed for Apple (CIK: 320193)...");
    let feed_options = FeedOptions::new(None)
        .with_param("count", "10")
        .with_param("type", "10-K");

    let company_feed = edgar.company_feed("320193", Some(feed_options)).await?;
    println!("✓ Company: {}", company_feed.title);

    if let Some(company_info) = &company_feed.company_info {
        println!("✓ Conformed Name: {}", company_info.conformed_name);
        println!(
            "✓ SIC: {} - {}",
            company_info.assigned_sic, company_info.assigned_sic_desc
        );
    }

    println!("✓ Found {} filings", company_feed.entries.len());

    // Example 3: SEC press releases
    println!("\n3. Fetching SEC press releases...");
    let press_releases = edgar.press_release_feed().await?;
    println!("✓ Feed: {}", press_releases.channel.title);
    println!(
        "✓ Recent press releases: {}",
        press_releases.channel.items.len()
    );

    if let Some(first_item) = press_releases.channel.items.first() {
        println!("\n   Latest Press Release:");
        println!("   - {}", first_item.title);
        println!(
            "   - Published: {}",
            first_item.pub_date.as_deref().unwrap_or("N/A")
        );
    }

    // Example 4: Administrative proceedings feed
    println!("\n4. Fetching administrative proceedings feed...");
    let proceedings = edgar.administrative_proceedings_feed().await?;
    println!("✓ Feed: {}", proceedings.channel.title);
    println!("✓ Items: {}", proceedings.channel.items.len());

    if !proceedings.channel.items.is_empty() {
        println!("\n   Recent proceedings items:");
        for (i, item) in proceedings.channel.items.iter().take(3).enumerate() {
            println!("   {}. {}", i + 1, item.title);
        }
    }

    // Example 5: XBRL feeds
    println!("\n5. Fetching inline XBRL feed...");
    let xbrl_feed = edgar.inline_xbrl_feed().await?;
    println!("✓ Feed: {}", xbrl_feed.channel.title);
    println!("✓ XBRL filings: {}", xbrl_feed.channel.items.len());

    // Example 6: Historical XBRL feed
    println!("\n6. Fetching historical XBRL feed (January 2024)...");
    let historical_xbrl = edgar.historical_xbrl_feed(2024, 1).await?;
    println!("✓ Feed: {}", historical_xbrl.channel.title);
    println!(
        "✓ Historical items: {}",
        historical_xbrl.channel.items.len()
    );

    println!("\n✓ RSS/Atom feeds examples completed successfully!");
    println!("\nNote: These feeds are updated regularly by the SEC.");
    println!("For real-time monitoring, consider polling these feeds periodically.");

    Ok(())
}
