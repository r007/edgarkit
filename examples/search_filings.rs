//! Search for SEC filings example
//!
//! This example demonstrates how to search for filings using the EdgarKit search API:
//! - Search by keywords and form types
//! - Filter by date range
//! - Handle pagination automatically
//!
//! Run with: `cargo run --example search_filings --all-features`

use edgarkit::{Edgar, SearchOperations, SearchOptions};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let edgar = Edgar::new("EdgarKit Example user@example.com")?;

    println!("=== EdgarKit Search Example ===\n");

    // Example 1: Search for recent 10-K filings
    println!("1. Searching for recent 10-K filings...");
    let options = SearchOptions::new()
        .with_forms(vec!["10-K".to_string()])
        .with_count(5); // Limit to 5 results for the example

    let response = edgar.search(options).await?;
    println!("✓ Found {} total 10-K filings", response.hits.total.value);
    println!("✓ Showing first {} results:\n", response.hits.hits.len());

    for (i, hit) in response.hits.hits.iter().enumerate() {
        println!(
            "   {}. {} - {} (filed: {})",
            i + 1,
            hit._source
                .display_names
                .first()
                .unwrap_or(&"Unknown".to_string()),
            hit._source.form,
            hit._source.file_date
        );
    }

    // Example 2: Search for SPAC-related S-1 filings with date range
    println!("\n2. Searching for SPAC-related S-1 filings...");
    let spac_options = SearchOptions::new()
        .with_query(r#""Special Purpose Acquisition""#)
        .with_forms(vec!["S-1".to_string()])
        .with_date_range("2024-01-01".to_string(), "2024-12-31".to_string())
        .with_count(10);

    let spac_response = edgar.search(spac_options).await?;
    println!(
        "✓ Found {} SPAC S-1 filings in 2024",
        spac_response.hits.total.value
    );

    // Example 3: Use search_all for automatic pagination
    println!("\n3. Demonstrating automatic pagination with search_all...");
    let pagination_options = SearchOptions::new()
        .with_query("Apple")
        .with_forms(vec!["8-K".to_string()])
        .with_count(100); // Request 100 per page

    println!("   Fetching all results (this may take a moment)...");
    let all_results = edgar.search_all(pagination_options).await?;
    println!(
        "✓ Retrieved {} total filings across all pages",
        all_results.len()
    );

    println!("\n✓ Search examples completed successfully!");

    Ok(())
}
