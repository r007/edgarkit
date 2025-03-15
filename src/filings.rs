use super::error::{EdgarError, Result};
use super::options::FilingOptions;
use super::traits::FilingOperations;
use super::Edgar;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use serde_json;

#[derive(Debug, Clone, Deserialize)]
pub struct Submission {
    pub cik: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub sic: String,
    #[serde(rename = "sicDescription")]
    pub sic_description: String,
    #[serde(rename = "insiderTransactionForOwnerExists")]
    pub insider_transaction_for_owner_exists: i32,
    #[serde(rename = "insiderTransactionForIssuerExists")]
    pub insider_transaction_for_issuer_exists: i32,
    pub name: String,
    pub tickers: Vec<String>,
    pub exchanges: Vec<String>,
    pub ein: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    #[serde(rename = "investmentCompany")]
    pub investment_company: Option<String>,
    pub category: Option<String>,
    #[serde(rename = "fiscalYearEnd")]
    pub fiscal_year_end: String,
    #[serde(rename = "stateOfIncorporation")]
    pub state_of_incorporation: String,
    #[serde(rename = "stateOfIncorporationDescription")]
    pub state_of_incorporation_description: String,
    pub addresses: Addresses,
    pub phone: String,
    pub flags: String,
    #[serde(rename = "formerNames")]
    pub former_names: Vec<FormerName>,
    pub filings: FilingsData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Addresses {
    pub mailing: Address,
    pub business: Address,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Address {
    pub street1: String,
    pub street2: Option<String>,
    pub city: String,
    #[serde(rename = "stateOrCountry")]
    pub state_or_country: String,
    #[serde(rename = "zipCode")]
    pub zip_code: String,
    #[serde(rename = "stateOrCountryDescription")]
    pub state_or_country_description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormerName {
    pub name: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilingsData {
    pub recent: RecentFilings,
    pub files: Vec<FilingFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilingFile {
    pub name: String,
    #[serde(rename = "filingCount")]
    pub filing_count: u64,
    #[serde(rename = "filingFrom")]
    pub filing_from: String,
    #[serde(rename = "filingTo")]
    pub filint_to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecentFilings {
    #[serde(rename = "accessionNumber")]
    pub accession_number: Vec<String>,
    #[serde(rename = "filingDate")]
    pub filing_date: Vec<String>,
    #[serde(rename = "reportDate")]
    pub report_date: Option<Vec<String>>,
    #[serde(rename = "acceptanceDateTime")]
    pub acceptance_date_time: Vec<String>,
    pub act: Option<Vec<String>>,
    pub form: Vec<String>,
    #[serde(rename = "fileNumber")]
    pub file_number: Option<Vec<String>>,
    #[serde(rename = "filmNumber")]
    pub film_number: Option<Vec<String>>,
    pub items: Option<Vec<String>>,
    pub size: Vec<i32>,
    #[serde(rename = "isXBRL")]
    pub is_xbrl: Option<Vec<i32>>,
    #[serde(rename = "isInlineXBRL")]
    pub is_inline_xbrl: Option<Vec<i32>>,
    #[serde(rename = "primaryDocument")]
    pub primary_document: Option<Vec<String>>,
    #[serde(rename = "primaryDocDescription")]
    pub primary_doc_description: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DetailedFiling {
    pub accession_number: String,
    pub filing_date: String,
    pub report_date: Option<String>,
    pub acceptance_date_time: DateTime<FixedOffset>,
    pub act: Option<String>,
    pub form: String,
    pub file_number: Option<String>,
    pub film_number: Option<String>,
    pub items: Option<String>,
    pub size: i32,
    pub is_xbrl: bool,
    pub is_inline_xbrl: bool,
    pub primary_document: Option<String>,
    pub primary_doc_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DirectoryResponse {
    pub directory: Directory,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Directory {
    pub item: Vec<DirectoryItem>,
    pub name: String,
    #[serde(rename = "parent-dir")]
    pub parent_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DirectoryItem {
    #[serde(rename = "last-modified")]
    pub last_modified: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: String,
}

impl RecentFilings {
    fn get_vec_item_at<T: Clone>(&self, vec_opt: &Option<Vec<T>>, idx: usize) -> Option<T> {
        vec_opt.as_ref().and_then(|v| v.get(idx).cloned())
    }

    fn get_bool_at(&self, vec_opt: &Option<Vec<i32>>, idx: usize) -> bool {
        vec_opt.as_ref().map(|x| x[idx] == 1).unwrap_or(false)
    }
}

impl TryFrom<(&RecentFilings, usize)> for DetailedFiling {
    type Error = chrono::ParseError;

    fn try_from(
        (recent, idx): (&RecentFilings, usize),
    ) -> std::result::Result<Self, chrono::ParseError> {
        let acceptance_date_time = DateTime::parse_from_rfc3339(&recent.acceptance_date_time[idx])?;

        Ok(DetailedFiling {
            accession_number: recent.accession_number[idx].clone(),
            filing_date: recent.filing_date[idx].clone(),
            report_date: recent.get_vec_item_at(&recent.report_date, idx),
            acceptance_date_time,
            act: recent.get_vec_item_at(&recent.act, idx),
            form: recent.form[idx].clone(),
            file_number: recent.get_vec_item_at(&recent.file_number, idx),
            film_number: recent.get_vec_item_at(&recent.film_number, idx),
            items: recent.get_vec_item_at(&recent.items, idx),
            size: recent.size[idx],
            is_xbrl: recent.get_bool_at(&recent.is_xbrl, idx),
            is_inline_xbrl: recent.get_bool_at(&recent.is_inline_xbrl, idx),
            primary_document: recent.get_vec_item_at(&recent.primary_document, idx),
            primary_doc_description: recent.get_vec_item_at(&recent.primary_doc_description, idx),
        })
    }
}

#[derive(Debug)]
enum UrlType {
    Submission,
    FilingDirectory,
    EntityDirectory,
    FilingContent,
    TextFiling,
}

impl Edgar {
    fn build_url(&self, url_type: UrlType, params: &[&str]) -> Result<String> {
        match url_type {
            UrlType::Submission => {
                let cik = format!("{:0>10}", params[0]);
                Ok(format!(
                    "{}/submissions/CIK{}.json",
                    self.edgar_data_url, cik
                ))
            }
            UrlType::FilingDirectory => {
                let (cik, acc_no) = (params[0], params[1]);
                let formatted_acc = acc_no.replace("-", "");
                Ok(format!(
                    "{}/data/{}/{}/index.json",
                    self.edgar_archives_url, cik, formatted_acc
                ))
            }
            UrlType::EntityDirectory => {
                let cik = format!("{:0>10}", params[0]);
                Ok(format!(
                    "{}/data/{}/index.json",
                    self.edgar_archives_url, cik
                ))
            }
            UrlType::FilingContent => {
                let (cik, acc_no, filename) = (params[0], params[1], params[2]);
                let formatted_acc = acc_no.replace("-", "");
                Ok(format!(
                    "{}/data/{}/{}/{}",
                    self.edgar_archives_url, cik, formatted_acc, filename
                ))
            }
            UrlType::TextFiling => {
                // For text filings: format is /Archives/edgar/data/CIK/ACC_NO_NO_DASHES/ACC_NO_WITH_DASHES.txt
                let (cik, acc_no) = (params[0], params[1]);
                let formatted_acc = acc_no.replace("-", "");
                Ok(format!(
                    "{}/data/{}/{}/{}.txt",
                    self.edgar_archives_url, cik, formatted_acc, acc_no
                ))
            }
        }
    }

    fn get_filing_url(&self, cik: &str, accession_number: &str, filename: &str) -> Result<String> {
        self.build_url(UrlType::FilingContent, &[cik, accession_number, filename])
    }

    // Add a convenience method to get text filing URL directly
    fn get_text_filing_url(&self, cik: &str, accession_number: &str) -> Result<String> {
        self.build_url(UrlType::TextFiling, &[cik, accession_number])
    }
}

/// Implementation of filing operations for the Edgar system.
///
/// This implementation provides methods to interact with the SEC's EDGAR system,
/// allowing retrieval of various filing-related data including submissions,
/// filing directories, and filing contents.
///
/// # Examples
///
/// ```
/// # use your_crate::{Edgar, FilingOperations};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let edgar = Edgar::new();
///
/// // Get recent filings for a company
/// let filings = edgar.get_recent_filings("1018724").await?;
///
/// // Get latest 10-K filing
/// let latest_10k = edgar.get_latest_filing_content("1018724", "10-K").await?;
/// # Ok(())
/// # }
/// ```
///
/// # Methods
///
/// - `submissions`: Retrieves submission history for a given CIK
/// - `get_recent_filings`: Gets recent filings for a company
/// - `filings`: Gets filtered filings based on provided options
/// - `filing_directory`: Retrieves the filing directory for a specific filing
/// - `entity_directory`: Gets the entity directory for a CIK
/// - `get_filing_url`: Constructs URLs for accessing filings
/// - `get_filing_url_from_id`: Constructs filing URLs from combined filing IDs
/// - `get_filing_content`: Retrieves the content of a specific filing
/// - `get_latest_filing_content`: Gets the most recent filing of a specified type
///
/// # Errors
///
/// Methods may return various error types wrapped in `Result`:
/// - `EdgarError::NotFound`: When requested data is not found
/// - `EdgarError::InvalidResponse`: When response data is malformed
/// - Network-related errors during HTTP requests
#[async_trait]
impl FilingOperations for Edgar {
    /// Retrieves submission history for a given CIK.
    ///
    /// This function sends an HTTP GET request to the SEC's EDGAR system to fetch the submission details
    /// for the specified Central Index Key (CIK). The function then parses the JSON response and returns
    /// a `Result` containing the parsed `Submission` struct.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice representing the Central Index Key (CIK) of the company.
    ///
    /// # Returns
    ///
    /// * `Result<Submission>` - A `Result` containing the parsed `Submission` struct if the operation is successful.
    ///   If an error occurs, it returns an `Err` containing the error.
    ///
    /// # Errors
    ///
    /// * `EdgarError::NotFound` - If the submission details for the given CIK are not found.
    /// * `EdgarError::InvalidResponse` - If the response data is malformed.
    /// * Network-related errors during HTTP requests.
    async fn submissions(&self, cik: &str) -> Result<Submission> {
        let url = self.build_url(UrlType::Submission, &[cik])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str::<Submission>(&response)?)
    }

    /// Retrieves recent filings for a given CIK.
    ///
    /// This function fetches the submission details for the given CIK,
    /// then processes the recent filings data to create a vector of `DetailedFiling` structs.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string representing the Central Index Key (CIK) of the company.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<DetailedFiling>>` - A `Result` containing a vector of `DetailedFiling` structs
    ///   if the operation is successful. If an error occurs, it returns an `Err` containing the error.
    ///
    /// # Errors
    ///
    /// * `EdgarError::NotFound` - If the submission details for the given CIK are not found.
    /// * `EdgarError::InvalidResponse` - If the submission details or recent filings data are invalid.
    async fn get_recent_filings(&self, cik: &str) -> Result<Vec<DetailedFiling>> {
        let submission = self.submissions(cik).await?;
        let mut detailed_filings = Vec::new();

        // Process recent filings
        for idx in 0..submission.filings.recent.accession_number.len() {
            if let Ok(filing) = DetailedFiling::try_from((&submission.filings.recent, idx)) {
                detailed_filings.push(filing);
            }
        }

        Ok(detailed_filings)
    }

    /// Retrieves and filters filings for a given company based on specified options.
    ///
    /// This function fetches all recent filings for a company identified by its CIK,
    /// and then applies optional filters such as form type, offset, and limit.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice containing the Central Index Key (CIK) of the company.
    /// * `opts` - An optional `FilingOptions` struct that specifies filtering criteria:
    ///   - `form_types`: If provided, only filings of these types will be returned.
    ///   - `offset`: If provided, skips this many filings from the start of the list.
    ///   - `limit`: If provided, returns at most this many filings.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `DetailedFiling` structs if successful.
    /// The vector represents the filtered list of filings based on the provided options.
    /// If an error occurs during the process, it returns an `Err` variant.
    async fn filings(&self, cik: &str, opts: Option<FilingOptions>) -> Result<Vec<DetailedFiling>> {
        let mut all_filings = self.get_recent_filings(cik).await?;

        // Apply filters if provided
        if let Some(opts) = opts {
            // Filter by form types if specified
            if let Some(ref form_types) = opts.form_types {
                all_filings.retain(|filing| form_types.iter().any(|ft| ft == &filing.form.trim()));
            }

            // Apply offset
            if let Some(offset) = opts.offset {
                all_filings = all_filings.into_iter().skip(offset).collect();
            }

            // Apply limit
            if let Some(limit) = opts.limit {
                all_filings.truncate(limit);
            }
        }

        Ok(all_filings)
    }

    /// Retrieves the filing directory for a specific filing.
    ///
    /// The filing directory contains a list of files associated with a specific filing.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string representing the Central Index Key (CIK) of the company.
    /// * `accession_number` - A string representing the accession number of the filing.
    ///
    /// # Returns
    ///
    /// * `Result<DirectoryResponse>` - A `Result` containing a `DirectoryResponse` struct
    ///   if the operation is successful. If an error occurs, it returns an `Err` containing the error.
    ///
    /// # Errors
    ///
    /// * `EdgarError::NotFound` - If the filing directory for the given CIK and accession number is not found.
    /// * `EdgarError::InvalidResponse` - If the response data is malformed.
    /// * Network-related errors during HTTP requests.
    async fn filing_directory(
        &self,
        cik: &str,
        accession_number: &str,
    ) -> Result<DirectoryResponse> {
        let url = self.build_url(UrlType::FilingDirectory, &[cik, accession_number])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str::<DirectoryResponse>(&response)?)
    }

    /// Retrieves the entity directory for a CIK.
    ///
    /// The entity directory contains a list of files associated with a specific company identified by its CIK.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string representing the Central Index Key (CIK) of the company.
    ///
    /// # Returns
    ///
    /// * `Result<DirectoryResponse>` - A `Result` containing a `DirectoryResponse` struct
    ///   if the operation is successful. If an error occurs, it returns an `Err` containing the error.
    ///
    /// # Errors
    ///
    /// * `EdgarError::NotFound` - If the entity directory for the given CIK is not found.
    /// * `EdgarError::InvalidResponse` - If the response data is malformed.
    /// * Network-related errors during HTTP requests.
    async fn entity_directory(&self, cik: &str) -> Result<DirectoryResponse> {
        let url = self.build_url(UrlType::EntityDirectory, &[cik])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str::<DirectoryResponse>(&response)?)
    }

    /// Retrieves the URL for accessing a specific filing based on the combined filing ID.
    ///
    /// The combined filing ID is expected to be in the format "accession_number:filename".
    /// This function splits the ID into its components, then constructs the URL for accessing the filing.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice representing the Central Index Key (CIK) of the company.
    /// * `filing_id` - A string slice representing the combined filing ID.
    ///
    /// # Returns
    ///
    /// * `Result<String>` - A `Result` containing a `String` with the URL for accessing the filing if successful.
    ///   If the filing ID format is invalid, it returns an `Err` containing an `EdgarError::InvalidResponse`.
    fn get_filing_url_from_id(&self, cik: &str, filing_id: &str) -> Result<String> {
        let parts: Vec<&str> = filing_id.split(":").collect();
        if parts.len() != 2 {
            return Err(EdgarError::InvalidResponse(
                "Invalid filing ID format. Expected 'accession_number:filename'".to_string(),
            ));
        }
        Ok(self.get_filing_url(cik, parts[0], parts[1])?)
    }

    /// Retrieves the content of a specific filing based on the combined filing ID.
    ///
    /// This function constructs the URL for accessing the filing using the provided CIK and filing ID,
    /// then fetches the content of the filing by making an HTTP GET request to the constructed URL.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice representing the Central Index Key (CIK) of the company.
    /// * `filing_id` - A string slice representing the combined filing ID.
    ///
    /// # Returns
    ///
    /// * `Result<String>` - A `Result` containing a `String` with the content of the filing if successful.
    ///   If an error occurs during the process, it returns an `Err` containing the error.
    async fn get_filing_content_by_id(&self, cik: &str, filing_id: &str) -> Result<String> {
        let url = self.get_filing_url_from_id(cik, filing_id)?;
        self.get(&url).await
    }

    /// Retrieves the content of the latest filing of a specified type for a given company.
    ///
    /// This function fetches the most recent filing of the specified form type for the company
    /// identified by the given CIK. It then retrieves and returns the content of that filing.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice containing the Central Index Key (CIK) of the company.
    /// * `form_type` - A string slice specifying the type of form to retrieve (e.g., "10-K", "10-Q").
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `String` with the content of the latest filing if successful.
    /// If an error occurs during the process, it returns an `Err` variant.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * No filings of the specified type are found for the given CIK.
    /// * The retrieved filing does not have a primary document.
    /// * There's an issue constructing the URL for the filing.
    /// * There's a problem fetching the content of the filing.
    async fn get_latest_filing_content(&self, cik: &str, form_type: &str) -> Result<String> {
        let opts = FilingOptions::new()
            .with_form_type(form_type.to_string())
            .with_limit(1);
        let filings = self.filings(cik, Some(opts)).await?;
        let filing = filings.first().ok_or_else(|| EdgarError::NotFound)?;
        let primary_doc = filing
            .primary_document
            .as_ref()
            .ok_or_else(|| EdgarError::InvalidResponse("No primary document found".to_string()))?;
        let url = self.get_filing_url(cik, &filing.accession_number, primary_doc)?;
        self.get(&url).await
    }

    /// Generates URLs for text filings based on specified options without downloading content
    ///
    /// This function fetches filings metadata for a company identified by its CIK,
    /// filters them based on the provided options, and then generates URLs for accessing
    /// the raw text versions of these filings.
    ///
    /// # Parameters
    ///
    /// * `cik` - A string slice containing the Central Index Key (CIK) of the company
    /// * `opts` - An optional `FilingOptions` struct that specifies filtering criteria
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of tuples with (DetailedFiling, text_url).
    /// If an error occurs during the process, it returns an `Err` variant.
    async fn get_text_filing_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String)>> {
        // Get filings based on options
        let filings = self.filings(cik, opts).await?;

        // Create a vector to hold the results
        let mut results = Vec::with_capacity(filings.len());

        // For each filing, generate the text filing URL
        for filing in filings {
            match self.get_text_filing_url(cik, &filing.accession_number) {
                Ok(url) => results.push((filing, url)),
                Err(err) => {
                    // Log the error but continue with other filings
                    tracing::warn!(
                        "Failed to generate URL for filing {}: {}",
                        filing.accession_number,
                        err
                    );
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SUBMISSION_FIXTURE: &str = "fixtures/submissions/submission.json";

    #[test]
    fn test_datetime_parsing() {
        let sample_dates = vec![
            "2015-06-01T07:06:52.000Z",
            "2015-05-29T18:54:18.000Z",
            "2015-05-29T18:53:07.000Z",
        ];

        for date in sample_dates {
            let parsed = DateTime::parse_from_rfc3339(&date);
            assert!(parsed.is_ok());
        }
    }

    #[test]
    fn test_parse_submission() {
        let content = fs::read_to_string(SUBMISSION_FIXTURE).unwrap();
        let submission: Submission = serde_json::from_str(&content).unwrap();

        assert_eq!(submission.name, "Apple Inc.");
        assert_eq!(submission.cik, "0000320193");
        assert_eq!(submission.tickers, vec!["AAPL"]);
        assert!(!submission.filings.recent.accession_number.is_empty());
    }

    #[test]
    fn test_detailed_filing_conversion() {
        let content = fs::read_to_string(SUBMISSION_FIXTURE).unwrap();
        let submission: Submission = serde_json::from_str(&content).unwrap();

        let filing = DetailedFiling::try_from((&submission.filings.recent, 0)).unwrap();

        assert!(filing.acceptance_date_time.timestamp() > 0);
        assert!(!filing.accession_number.is_empty());
        assert!(!filing.filing_date.is_empty());
    }

    #[test]
    fn test_parse_directory() {
        let content = fs::read_to_string("fixtures/submissions/directory.json").unwrap();
        let dir: DirectoryResponse = serde_json::from_str(&content).unwrap();

        assert!(!dir.directory.item.is_empty());
        let first_item = &dir.directory.item[0];
        assert_eq!(first_item.name, "0001140361-25-000228-index-headers.html");
        assert_eq!(first_item.type_, "text.gif");
    }

    #[tokio::test]
    async fn test_latest_filing_content() {
        // Create Edgar instance
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        // Test fetching 10-K filing
        let filing_content = edgar
            .get_latest_filing_content("320193", "10-K")
            .await
            .unwrap();

        // Basic content validation
        assert!(!filing_content.is_empty());
        assert!(filing_content.len() > 1000); // Should have substantial content

        // Test with invalid CIK
        let invalid_result = edgar.get_latest_filing_content("000000", "10-K").await;
        assert!(matches!(invalid_result, Err(EdgarError::NotFound)));

        // Test with invalid form type
        let invalid_form = edgar.get_latest_filing_content("320193", "INVALID").await;
        assert!(matches!(invalid_form, Err(EdgarError::NotFound)));
    }

    #[tokio::test]
    async fn test_get_text_filing_links() {
        // Create Edgar instance
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        // Test with Apple Inc. CIK and limit to 3 filings
        let opts = FilingOptions::new().with_limit(3);
        let filing_links = edgar
            .get_text_filing_links("320193", Some(opts))
            .await
            .unwrap();

        // Verify we got the right number of links
        assert_eq!(
            filing_links.len(),
            3,
            "Should return exactly 3 filing links"
        );

        // Verify each link is properly formatted for text filings
        for (filing, url) in &filing_links {
            // URLs should follow format: {base}/data/{cik}/{acc_no_without_dashes}/{acc_no_with_dashes}.txt
            let expected_url_pattern = format!(
                "{}/data/320193/{}/{}.txt",
                edgar.edgar_archives_url,
                filing.accession_number.replace("-", ""),
                filing.accession_number
            );

            assert_eq!(
                &expected_url_pattern, url,
                "Text filing URL is incorrectly formatted"
            );
        }

        // Test filtering by form type (10-K filings only)
        let form_opts = FilingOptions::new().with_form_type("10-K").with_limit(2);
        let form_filing_links = edgar
            .get_text_filing_links("320193", Some(form_opts))
            .await
            .unwrap();

        // Verify all returned filings are 10-K forms
        for (filing, _) in &form_filing_links {
            assert_eq!(
                filing.form, "10-K",
                "Filing form should be 10-K when filtered by form type"
            );
        }

        // Test with empty results
        let invalid_form_opts = FilingOptions::new().with_form_type("INVALID_FORM_TYPE");
        let invalid_form_result = edgar
            .get_text_filing_links("320193", Some(invalid_form_opts))
            .await
            .unwrap();
        assert!(
            invalid_form_result.is_empty(),
            "Should return empty results for non-existent form type"
        );
    }

    #[tokio::test]
    async fn test_text_filing_url_format() {
        // Create Edgar instance
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        // Test specific URL pattern with well-known CIK and accession number
        let cik = "1889983"; // Example CIK
        let accession_number = "0001213900-23-009668"; // Example accession number

        let url = edgar.get_text_filing_url(cik, accession_number).unwrap();

        // Expected URL pattern
        let formatted_acc = accession_number.replace("-", "");
        let expected_url = format!(
            "{}/data/{}/{}/{}.txt",
            edgar.edgar_archives_url, cik, formatted_acc, accession_number
        );

        assert_eq!(
            url, expected_url,
            "Text filing URL format doesn't match expected pattern"
        );
    }

    #[tokio::test]
    async fn test_filings_with_form_type() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let opts = FilingOptions::new().with_form_type("10-K");
        let filings = edgar.filings("320193", Some(opts)).await.unwrap();
        assert!(filings.iter().all(|f| f.form == "10-K"));
    }

    #[tokio::test]
    async fn test_filings_with_limit() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let opts = FilingOptions::new().with_limit(1);
        let filings = edgar.filings("320193", Some(opts)).await.unwrap();
        assert_eq!(filings.len(), 1);
    }

    #[tokio::test]
    async fn test_filings_with_offset() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let all_filings = edgar.filings("320193", None).await.unwrap();
        let opts = FilingOptions::new().with_offset(1);
        let offset_filings = edgar.filings("320193", Some(opts)).await.unwrap();
        assert_eq!(offset_filings.len(), all_filings.len() - 1);
    }

    #[tokio::test]
    async fn test_submissions() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let submissions = edgar.submissions("320193").await.unwrap();
        assert_eq!(submissions.name, "Apple Inc.");
    }

    #[tokio::test]
    async fn test_submissions_not_found() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.submissions("0").await;
        assert!(matches!(result, Err(EdgarError::NotFound)));
    }
}
