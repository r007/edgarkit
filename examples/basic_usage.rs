//! Basic EdgarKit usage example
//!
//! This example demonstrates the simplest way to use EdgarKit:
//! - Initialize the client
//! - Look up a company by ticker
//! - Retrieve company information
//!
//! Run with: `cargo run --example basic_usage --all-features`

use edgarkit::{CompanyOperations, Edgar};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the Edgar client with a proper user agent
    // SEC.gov requires a user agent in the format: "AppName contact@example.com"
    let edgar = Edgar::new("EdgarKit Example user@example.com")?;

    println!("=== EdgarKit Basic Usage Example ===\n");

    // Look up Apple's CIK (Central Index Key) by ticker symbol
    println!("Looking up company by ticker symbol...");
    let ticker = "AAPL";
    let cik = edgar.company_cik(ticker).await?;
    println!("✓ Found CIK for {}: {}\n", ticker, cik);

    // Retrieve company facts and metadata
    println!("Fetching company facts...");
    let facts = edgar.company_facts(cik).await?;
    println!("✓ Company Name: {}", facts.entity_name);
    println!("✓ CIK: {}", facts.cik);

    // Display some available taxonomies
    println!("\nAvailable data taxonomies:");
    println!("- US-GAAP facts: {} items", facts.taxonomies.us_gaap.len());
    println!("- DEI facts: {} items", facts.taxonomies.dei.len());

    println!("\n✓ Basic usage example completed successfully!");

    Ok(())
}
