//! Download company filings example
//!
//! This example demonstrates how to retrieve and download filings:
//! - Get a list of recent filings for a company
//! - Filter by form type
//! - Retrieve filing metadata
//! - Download filing content
//!
//! Run with: `cargo run --example download_filings --all-features`

use edgarkit::{Edgar, FilingOperations, FilingOptions};
use std::collections::HashMap;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let edgar = Edgar::new("EdgarKit Example user@example.com")?;

    println!("=== EdgarKit Filing Download Example ===\n");

    // Apple's CIK
    let cik = "320193";

    // Example 1: Get recent 10-K filings
    println!("1. Fetching recent 10-K filings for CIK {}...", cik);
    let options = FilingOptions::new().with_form_type("10-K").with_limit(3);

    let filings = edgar.filings(cik, Some(options)).await?;
    println!("✓ Found {} 10-K filings:\n", filings.len());

    for (i, filing) in filings.iter().enumerate() {
        println!("   {}. Form: {}", i + 1, filing.form);
        println!("      Filed: {}", filing.filing_date);
        println!("      Accession: {}", filing.accession_number);
        if let Some(doc) = &filing.primary_document {
            println!("      Primary Document: {}", doc);
        }
        println!();
    }

    // Example 2: Get the latest 10-K filing content
    println!("2. Downloading the latest 10-K filing content...");
    let filing_content = edgar.get_latest_filing_content(cik, "10-K").await?;

    println!("✓ Downloaded filing content:");
    println!("   Size: {} bytes", filing_content.len());
    println!("   Preview (first 200 chars):");
    println!("   {}", &filing_content[..200.min(filing_content.len())]);

    // Example 3: Get all submissions for a company
    println!("\n3. Fetching all submission data...");
    let submission = edgar.submissions(cik).await?;

    println!("✓ Company: {}", submission.name);
    println!("✓ SIC: {} - {}", submission.sic, submission.sic_description);
    println!(
        "✓ Fiscal Year End: {}",
        submission.fiscal_year_end.as_deref().unwrap_or("N/A")
    );
    println!(
        "✓ Total Recent Filings: {}",
        submission.filings.recent.accession_number.len()
    );

    // Example 4: Get filing directory
    println!("\n4. Retrieving filing directory...");
    if let Some(latest_filing) = filings.first() {
        let directory = edgar
            .filing_directory(cik, &latest_filing.accession_number)
            .await?;
        println!("✓ Directory for {}:", latest_filing.accession_number);
        println!("   Name: {}", directory.directory.name);
        println!("   Parent Directory: {}", directory.directory.parent_dir);
        println!("   Files in directory: {}", directory.directory.item.len());

        // Show first few files
        for (i, item) in directory.directory.item.iter().take(5).enumerate() {
            println!("   {}. {} ({})", i + 1, item.name, item.type_);
        }
    }

    // Example 5: Get multiple form types
    println!("\n5. Fetching multiple form types...");
    let multi_options = FilingOptions::new()
        .with_form_types(vec![
            "10-K".to_string(),
            "10-Q".to_string(),
            "8-K".to_string(),
        ])
        .with_limit(10);

    let multi_filings = edgar.filings(cik, Some(multi_options)).await?;
    println!("✓ Found {} filings of mixed types", multi_filings.len());

    // Count by form type
    let mut form_counts = HashMap::new();
    for filing in multi_filings {
        *form_counts.entry(filing.form.clone()).or_insert(0) += 1;
    }

    println!("   Distribution:");
    for (form, count) in form_counts {
        println!("   - {}: {}", form, count);
    }

    println!("\n✓ Filing download examples completed successfully!");

    Ok(())
}
