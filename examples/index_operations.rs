//! Index operations example
//!
//! This example demonstrates how to work with SEC EDGAR index files:
//! - Daily index files
//! - Quarterly index files
//! - Filtering index entries
//! - Parsing index data
//!
//! Run with: `cargo run --example index_operations --all-features`

use edgarkit::{Edgar, EdgarDay, EdgarPeriod, FilingOptions, IndexOperations, Quarter};
use std::collections::HashMap;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let edgar = Edgar::new("EdgarKit Example user@example.com")?;

    println!("=== EdgarKit Index Operations Example ===\n");

    // Example 1: Get daily filings for a specific date
    println!("1. Fetching daily filings for a specific date...");
    let day = EdgarDay::new(2024, 8, 15)?;

    let options = FilingOptions::new().with_form_type("10-K").with_limit(10);

    let daily_filings = edgar.get_daily_filings(day, Some(options)).await?;
    println!(
        "✓ Found {} 10-K filings on {}",
        daily_filings.len(),
        day.format_date()
    );

    for (i, entry) in daily_filings.iter().take(5).enumerate() {
        println!(
            "   {}. {} - {} (CIK: {})",
            i + 1,
            entry.company_name,
            entry.form_type,
            entry.cik
        );
    }

    // Example 2: Get quarterly index
    println!("\n2. Fetching quarterly index for Q3 2024...");
    let period = EdgarPeriod::new(2024, Quarter::Q3)?;
    let period_filings = edgar.get_period_filings(period, None).await?;
    println!("✓ Found {} filings in Q3 2024", period_filings.len());

    // Count filings by form type
    let mut form_counts = HashMap::new();
    for entry in &period_filings {
        *form_counts.entry(entry.form_type.trim()).or_insert(0) += 1;
    }

    println!("\n   Top form types in Q3 2024:");
    let mut sorted_forms: Vec<_> = form_counts.iter().collect();
    sorted_forms.sort_by(|a, b| b.1.cmp(a.1));
    for (form, count) in sorted_forms.iter().take(10) {
        println!("   - {}: {} filings", form, count);
    }

    // Example 3: Filter by multiple form types
    println!("\n3. Filtering for specific form types...");
    let multi_form_options = FilingOptions::new()
        .with_form_types(vec![
            "10-K".to_string(),
            "10-Q".to_string(),
            "S-1".to_string(),
        ])
        .with_limit(20);

    let filtered_daily = edgar
        .get_daily_filings(day, Some(multi_form_options))
        .await?;
    println!(
        "✓ Found {} filings of types 10-K, 10-Q, S-1",
        filtered_daily.len()
    );

    // Example 4: Get daily index response (directory listing)
    println!("\n4. Fetching daily index directory...");
    let daily_index = edgar.daily_index(Some(period)).await?;
    println!("✓ Index directory name: {}", daily_index.directory.name);
    println!("✓ Parent directory: {}", daily_index.directory.parent_dir);
    println!("✓ Files available: {}", daily_index.directory.item.len());

    println!("\n   Available index files:");
    for (i, item) in daily_index.directory.item.iter().take(10).enumerate() {
        println!("   {}. {} ({:?})", i + 1, item.name, item.type_);
    }

    // Example 5: Get full quarterly index
    println!("\n5. Fetching full quarterly index metadata...");
    let full_index = edgar.full_index(Some(period)).await?;
    println!("✓ Full index for: {}", full_index.directory.name);
    println!("✓ Available files: {}", full_index.directory.item.len());

    // Example 6: Filter by CIK
    println!("\n6. Filtering filings by specific CIK...");
    let cik_options = FilingOptions::new().with_cik(320193); // Apple

    let apple_filings = edgar.get_period_filings(period, Some(cik_options)).await?;
    println!(
        "✓ Found {} filings for Apple in Q3 2024",
        apple_filings.len()
    );

    for (i, filing) in apple_filings.iter().enumerate() {
        println!(
            "   {}. {} - filed on {}",
            i + 1,
            filing.form_type,
            filing.date_filed
        );
    }

    println!("\n✓ Index operations examples completed successfully!");
    println!("\nNote: Index files provide a comprehensive view of all filings");
    println!("but may be large. Consider using specific filters to reduce data transfer.");

    Ok(())
}
