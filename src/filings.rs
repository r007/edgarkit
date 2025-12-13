//! Company submissions, filing metadata, and filing content.
//!
//! EDGAR exposes a few different layers of filing data:
//! - **Submissions** (`/submissions/CIK##########.json`): a company-centric view that includes
//!   entity metadata plus a “recent filings” table.
//! - **Directory listings** (`/Archives/edgar/data/.../index.json`): a filing-centric view of the
//!   files that make up a specific submission (HTML, XML, exhibits, XBRL, etc.).
//! - **File content** (`/Archives/edgar/data/.../<filename>`): the actual primary document and
//!   related artifacts.
//!
//! The `FilingOperations` implementation in this module is designed for the common workflow:
//! resolve a CIK → list or filter filings → download a specific document. Filtering supports
//! form-type matching and can optionally include amendments (e.g., treating `10-K` as
//! `10-K` + `10-K/A`).

use super::Edgar;
use super::error::{EdgarError, Result};
use super::options::FilingOptions;
use super::traits::FilingOperations;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use serde_json;

/// A company's submissions payload (`/submissions/CIK##########.json`).
///
/// This is the primary metadata response for company-centric filing history. It includes a
/// “recent filings” section represented as parallel arrays, plus references to older filing
/// files when applicable.
#[derive(Debug, Clone, Deserialize)]
pub struct Submission {
    /// Zero-padded CIK (e.g., "0000320193")
    pub cik: String,

    /// Entity type (e.g., operating, investment)
    #[serde(rename = "entityType")]
    pub entity_type: String,

    /// Standard Industrial Classification code
    pub sic: String,

    /// Human-readable SIC description
    #[serde(rename = "sicDescription")]
    pub sic_description: String,

    /// Owner org type for insiders/issuers
    #[serde(rename = "ownerOrg")]
    pub owner_org: Option<String>,

    /// Insider transactions for owner
    #[serde(rename = "insiderTransactionForOwnerExists")]
    pub insider_transaction_for_owner_exists: i32,

    /// Insider transactions for issuer
    #[serde(rename = "insiderTransactionForIssuerExists")]
    pub insider_transaction_for_issuer_exists: i32,

    /// Conformed company name
    pub name: String,

    /// Exchange tickers (usually 1)
    pub tickers: Vec<String>,

    /// Exchanges for tickers, each corresponding to `tickers`
    pub exchanges: Vec<Option<String>>,

    /// Employer Identification Number
    pub ein: Option<String>,

    /// Legal Entity Identifier
    pub lei: Option<String>,

    /// Business description
    pub description: Option<String>,

    /// Company website
    pub website: Option<String>,

    /// Investor relations website
    #[serde(rename = "investorWebsite")]
    pub investor_website: Option<String>,

    /// Investment company flag/category
    #[serde(rename = "investmentCompany")]
    pub investment_company: Option<String>,

    /// Category (e.g., Large Accelerated Filer)
    pub category: Option<String>,

    /// Fiscal year end (e.g., 1231)
    #[serde(rename = "fiscalYearEnd")]
    pub fiscal_year_end: Option<String>,

    /// State code of incorporation
    #[serde(rename = "stateOfIncorporation")]
    pub state_of_incorporation: String,

    /// State full name
    #[serde(rename = "stateOfIncorporationDescription")]
    pub state_of_incorporation_description: String,

    /// Mailing and business addresses
    pub addresses: Addresses,

    /// Company phone
    pub phone: String,

    /// Misc flags
    pub flags: String,

    /// Historical names
    #[serde(rename = "formerNames")]
    pub former_names: Vec<FormerName>,

    /// Recent filings data
    pub filings: FilingsData,
}

/// Mailing and business addresses for an entity.
#[derive(Debug, Clone, Deserialize)]
pub struct Addresses {
    pub mailing: Address,
    pub business: Address,
}

/// A single address record in a `Submission` payload.
#[derive(Debug, Clone, Deserialize)]
pub struct Address {
    /// Street line 1
    pub street1: String,

    /// Street line 2
    pub street2: Option<String>,

    /// City
    pub city: String,

    /// State or country code
    #[serde(rename = "stateOrCountry")]
    pub state_or_country: Option<String>,

    /// Postal code
    #[serde(rename = "zipCode")]
    pub zip_code: Option<String>,

    /// Human-readable state or country
    #[serde(rename = "stateOrCountryDescription")]
    pub state_or_country_description: Option<String>,

    /// Foreign address flag
    #[serde(rename = "isForeignLocation")]
    pub is_foreign_location: Option<i32>,

    /// Foreign state/territory name
    #[serde(rename = "foreignStateTerritory")]
    pub foreign_state_territory: Option<String>,

    /// Country name
    pub country: Option<String>,

    /// ISO country code
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
}

/// A historical company name and the date range it was used.
#[derive(Debug, Clone, Deserialize)]
pub struct FormerName {
    pub name: String,
    pub from: String,
    pub to: String,
}

/// Filing history container in a `Submission` payload.
#[derive(Debug, Clone, Deserialize)]
pub struct FilingsData {
    pub recent: RecentFilings,
    pub files: Vec<FilingFile>,
}

/// Metadata for an older filing file segment referenced by a `Submission` payload.
#[derive(Debug, Clone, Deserialize)]
pub struct FilingFile {
    pub name: String,

    #[serde(rename = "filingCount")]
    pub filing_count: u64,

    #[serde(rename = "filingFrom")]
    pub filing_from: String,

    #[serde(rename = "filingTo")]
    pub filing_to: String,
}

/// “Recent filings” table from the submissions endpoint.
///
/// The SEC represents this data as parallel arrays (e.g., `accessionNumber[i]`, `form[i]`,
/// `filingDate[i]`) rather than a list of objects. Use `get_recent_filings` to convert this into
/// a list of [`DetailedFiling`] values.
#[derive(Debug, Clone, Deserialize)]
pub struct RecentFilings {
    /// EDGAR accession numbers
    #[serde(rename = "accessionNumber")]
    pub accession_number: Vec<String>,

    /// Filing dates (YYYY-MM-DD)
    #[serde(rename = "filingDate")]
    pub filing_date: Vec<String>,

    /// Report dates if provided
    #[serde(rename = "reportDate")]
    pub report_date: Option<Vec<String>>,

    /// EDGAR acceptance timestamps
    #[serde(rename = "acceptanceDateTime")]
    pub acceptance_date_time: Vec<String>,

    /// Securities Act references (e.g., 33, 34)
    pub act: Option<Vec<String>>,

    /// Form types (e.g., 10-K, 8-K)
    pub form: Vec<String>,

    /// File numbers, may be empty
    #[serde(rename = "fileNumber")]
    pub file_number: Option<Vec<String>>,

    /// Film numbers, may be empty
    #[serde(rename = "filmNumber")]
    pub film_number: Option<Vec<String>>,

    /// 8-K items strings (e.g., "1.01,2.03,5.01")
    pub items: Option<Vec<String>>,

    /// Document sizes in bytes
    pub size: Vec<i32>,

    /// XBRL flags (1 = has XBRL, 0 = no XBRL)
    #[serde(rename = "isXBRL")]
    pub is_xbrl: Option<Vec<i32>>,

    /// Inline XBRL flags (1 = has Inline XBRL, 0 = no Inline XBRL)
    #[serde(rename = "isInlineXBRL")]
    pub is_inline_xbrl: Option<Vec<i32>>,

    /// Primary document filenames
    #[serde(rename = "primaryDocument")]
    pub primary_document: Option<Vec<String>>,

    /// Primary document descriptions
    #[serde(rename = "primaryDocDescription")]
    pub primary_doc_description: Option<Vec<String>>,
}

/// A normalized filing record derived from the “recent filings” table.
///
/// Unlike `RecentFilings`, this struct is row-oriented and easier to work with in typical Rust
/// code. It is constructed via `TryFrom<(&RecentFilings, usize)>`.
#[derive(Debug, Clone)]
pub struct DetailedFiling {
    /// EDGAR accession number
    pub accession_number: String,

    /// Filing date (YYYY-MM-DD)
    pub filing_date: String,

    /// Report date (if any)
    pub report_date: Option<String>,

    /// EDGAR acceptance timestamp
    pub acceptance_date_time: DateTime<FixedOffset>,

    /// Securities Act reference (e.g., 33, 34)
    pub act: Option<String>,

    /// Form type
    pub form: String,

    /// File number
    pub file_number: Option<String>,

    /// Film number
    pub film_number: Option<String>,

    /// 8-K items string (e.g., "1.01,2.03,5.01")
    pub items: Option<String>,

    /// Document size in bytes
    pub size: i32,

    /// Contains XBRL
    pub is_xbrl: bool,

    /// Contains Inline XBRL
    pub is_inline_xbrl: bool,

    /// Primary document filename
    pub primary_document: Option<String>,

    /// Primary document description
    pub primary_doc_description: Option<String>,
}

/// Response wrapper for EDGAR `index.json` directory listings.
#[derive(Debug, Clone, Deserialize)]
pub struct DirectoryResponse {
    pub directory: Directory,
}

/// Directory listing payload for filings and entities.
#[derive(Debug, Clone, Deserialize)]
pub struct Directory {
    pub item: Vec<DirectoryItem>,
    pub name: String,
    #[serde(rename = "parent-dir")]
    pub parent_dir: String,
}

/// A file entry inside a directory listing.
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
    OriginalFiling,
    SgmlHeader,
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
            UrlType::OriginalFiling => {
                // For original filings: format is /Archives/edgar/data/CIK/ACC_NO_NO_DASHES/ACC_NO_WITH_DASHES-index.html
                let (cik, acc_no) = (params[0], params[1]);
                let formatted_acc = acc_no.replace("-", "");
                Ok(format!(
                    "{}/data/{}/{}/{}-index.html",
                    self.edgar_archives_url, cik, formatted_acc, acc_no
                ))
            }
            UrlType::SgmlHeader => {
                // For SGML headers: format is /Archives/edgar/data/CIK/ACC_NO_NO_DASHES/ACC_NO_WITH_DASHES.hdr.sgml
                let (cik, acc_no) = (params[0], params[1]);
                let formatted_acc = acc_no.replace("-", "");
                Ok(format!(
                    "{}/data/{}/{}/{}.hdr.sgml",
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

    // Add a convenience method to get original filing URL directly
    fn get_original_filing_url(&self, cik: &str, accession_number: &str) -> Result<String> {
        self.build_url(UrlType::OriginalFiling, &[cik, accession_number])
    }

    // Add a convenience method to get SGML header URL directly
    fn get_sgml_header_url(&self, cik: &str, accession_number: &str) -> Result<String> {
        self.build_url(UrlType::SgmlHeader, &[cik, accession_number])
    }
}

/// Filing operations for EDGAR submissions and filing content.
///
/// This implementation is built around the SEC “submissions” endpoint, which is the canonical
/// source for a company’s recent filing history. The raw payload represents filings as *parallel
/// arrays*; edgarkit converts that into a list of [`DetailedFiling`] values so it’s easy to filter,
/// paginate, and download documents.
///
/// **What you typically do:**
/// 1) Call `filings()` (or `get_recent_filings()`) to get metadata.
/// 2) Use `filing_directory()` to discover the files for a specific accession.
/// 3) Download the primary document with `get_latest_filing_content()` or `get_filing_content_by_id()`.
///
/// **Behavior notes:**
/// - `filings()` filters in-memory and returns results in the same order as the SEC provides
///   (typically newest-first).
/// - When converting the SEC parallel arrays into rows, entries with invalid timestamps are
///   skipped rather than failing the entire call.
/// - If you filter by form types, amendments can be included automatically via
///   [`FilingOptions::with_include_amendments`] (enabled by default).
#[async_trait]
impl FilingOperations for Edgar {
    /// Retrieves submission history for a given CIK.
    ///
    /// This is the raw `submissions/CIK##########.json` payload: entity metadata plus a recent
    /// filings table represented as parallel arrays.
    ///
    /// The SEC expects a zero-padded CIK in the URL; edgarkit handles that formatting for you.
    ///
    /// # Errors
    /// Returns an error if the company is not found, the response is not valid JSON, or the
    /// request fails.
    async fn submissions(&self, cik: &str) -> Result<Submission> {
        let url = self.build_url(UrlType::Submission, &[cik])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str::<Submission>(&response)?)
    }

    /// Retrieves recent filings for a given CIK.
    ///
    /// This is a convenience wrapper around `submissions()` that normalizes the SEC “recent” table
    /// into row-oriented [`DetailedFiling`] records.
    ///
    /// If a specific row has an invalid timestamp (e.g., malformed `acceptanceDateTime`), that row is
    /// skipped; the rest of the results are returned.
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

    /// Gets filings for a company, with optional filtering by form type, offset, and limit.
    ///
    /// By default, when you request a specific form type like "S-1", this automatically includes
    /// amendments too ("S-1/A") so you get the complete picture. You can disable this with
    /// `with_include_amendments(false)`.
    ///
    /// Results are returned in the same order as the SEC submissions payload (typically newest-first).
    ///
    /// # Parameters
    ///
    /// * `cik` - The company's Central Index Key
    /// * `opts` - Optional filters:
    ///   - `form_types`: Which form types to include
    ///   - `include_amendments`: Whether to add amendment forms automatically (default: true)
    ///   - `offset`: Skip this many filings from the start
    ///   - `limit`: Return at most this many filings
    ///
    /// # Returns
    ///
    /// A list of filings matching your criteria, sorted with newest first.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use edgarkit::{Edgar, FilingOperations, FilingOptions};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let edgar = Edgar::new("app contact@example.com")?;
    ///
    ///     // Returns both S-1 and S-1/A filings (default behavior).
    ///     let opts = FilingOptions::new().with_form_type("S-1".to_string());
    ///     let filings = edgar.filings("320193", Some(opts)).await?;
    ///
    ///     // Returns only S-1 filings, excluding amendments.
    ///     let opts = FilingOptions::new()
    ///         .with_form_type("S-1".to_string())
    ///         .with_include_amendments(false);
    ///     let filings_no_amends = edgar.filings("320193", Some(opts)).await?;
    ///
    ///     println!("with_amendments={}, without_amendments={}", filings.len(), filings_no_amends.len());
    ///     Ok(())
    /// }
    /// ```
    async fn filings(&self, cik: &str, opts: Option<FilingOptions>) -> Result<Vec<DetailedFiling>> {
        let mut all_filings = self.get_recent_filings(cik).await?;

        // Apply filters if provided
        if let Some(opts) = opts {
            // Filter by form types if specified
            if let Some(ref form_types) = opts.form_types {
                let mut expanded_types = form_types.clone();

                // Add amendment forms if include_amendments is true (default)
                if opts.include_amendments {
                    for form_type in form_types {
                        if !form_type.ends_with("/A") {
                            expanded_types.push(format!("{}/A", form_type));
                        }
                    }
                }

                all_filings
                    .retain(|filing| expanded_types.iter().any(|ft| ft == &filing.form.trim()));
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
    /// The filing directory is an `index.json` listing of the files that make up an accession.
    /// Use this when you need to locate the primary document, exhibits, XBRL artifacts, or other
    /// filenames before downloading content.
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
    /// The entity directory is an `index.json` listing at the company level. It is useful for
    /// browsing what is present under `/Archives/edgar/data/<CIK>/`.
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
    /// `filing_id` is expected to be in the format `"<accession_number>:<filename>"`.
    /// This is a compact way to pass around a specific document reference (for example, when
    /// you store `IndexEntry` hits and later want to fetch a specific file).
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
    /// This is a convenience wrapper around `get_filing_url_from_id()` plus a download.
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

    /// Downloads the content of the most recent filing of a specific type.
    ///
    /// This gets you the actual document content, not just metadata. By default, when you ask for
    /// a form type like "10-K", it includes amendments ("10-K/A") automatically and returns the
    /// most recent matching filing, which could be either the original or an amendment.
    ///
    /// # Parameters
    ///
    /// * `cik` - The company's Central Index Key
    /// * `form_type` - The form type you want (e.g., "10-K", "S-1")
    ///
    /// # Returns
    ///
    /// The filing content as a string (usually HTML or XML).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * No filings of that type exist for this company
    /// * The filing doesn't have a primary document
    /// * There's a network issue downloading the content
    ///
    /// # Example
    ///
    /// ```ignore
    /// use edgarkit::{Edgar, FilingOperations};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let edgar = Edgar::new("app contact@example.com")?;
    ///
    ///     // Gets the latest 10-K or 10-K/A filing.
    ///     let content = edgar.get_latest_filing_content("320193", "10-K").await?;
    ///     println!("Downloaded {} bytes", content.len());
    ///     Ok(())
    /// }
    /// ```
    async fn get_latest_filing_content(&self, cik: &str, form_type: &str) -> Result<String> {
        let opts = FilingOptions::new().with_form_type(form_type.to_string());
        let filings = self.filings(cik, Some(opts)).await?;

        // Get the first filing - it's already the most recent since filings() returns them sorted
        // and includes amendments automatically
        let filing = filings.first().ok_or(EdgarError::NotFound)?;

        let primary_doc = filing
            .primary_document
            .as_ref()
            .ok_or_else(|| EdgarError::InvalidResponse("No primary document found".to_string()))?;

        let url = self.get_filing_url(cik, &filing.accession_number, primary_doc)?;
        self.get(&url).await
    }

    /// Generates download and browser links for the *text* rendition of filings.
    ///
    /// This does not download any filing content. It returns tuples of:
    /// - [`DetailedFiling`] metadata
    /// - an EDGAR archives URL for the raw text file (`.../<accession>.txt`)
    /// - an SEC.gov “index page” URL suitable for browsing in a web browser
    ///
    /// Use this when you want to hand off URLs to another system (queue, downloader, UI) without
    /// eagerly fetching the documents.
    async fn get_text_filing_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String, String)>> {
        let filings = self.filings(cik, opts).await?;

        let mut links = Vec::new();
        for filing in filings {
            // Get text filing URL (for downloading raw content)
            let text_url = self.get_text_filing_url(cik, &filing.accession_number)?;

            // Get original SEC.gov URL (for web browsing)
            let sec_gov_url = self.get_original_filing_url(cik, &filing.accession_number)?;

            links.push((filing, text_url, sec_gov_url));
        }

        Ok(links)
    }

    /// Generates download and browser links for SGML header (`.hdr.sgml`) files.
    ///
    /// Like `get_text_filing_links()`, this is a link builder: it filters filings and returns
    /// URLs, but does not download anything.
    async fn get_sgml_header_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String, String)>> {
        let filings = self.filings(cik, opts).await?;

        let mut links = Vec::new();
        for filing in filings {
            // Get SGML header URL (for downloading header content)
            let sgml_url = self.get_sgml_header_url(cik, &filing.accession_number)?;

            // Get original SEC.gov URL (for web browsing)
            let sec_gov_url = self.get_original_filing_url(cik, &filing.accession_number)?;

            links.push((filing, sgml_url, sec_gov_url));
        }

        Ok(links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SUBMISSION_FIXTURE: &str = "../fixtures/submissions/submission.json";

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
        let content = fs::read_to_string("../fixtures/submissions/directory.json").unwrap();
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
        for (filing, url, sec_url) in &filing_links {
            // URLs should follow format: {base}/data/{cik}/{acc_no_without_dashes}/{acc_no_with_dashes}.txt
            let expected_url_pattern = format!(
                "{}/data/320193/{}/{}.txt",
                edgar.edgar_archives_url,
                filing.accession_number.replace("-", ""),
                filing.accession_number
            );

            let expected_sec_url_pattern = format!(
                "{}/data/320193/{}/{}-index.html",
                edgar.edgar_archives_url,
                filing.accession_number.replace("-", ""),
                filing.accession_number
            );

            assert_eq!(
                &expected_url_pattern, url,
                "Text filing URL is incorrectly formatted"
            );

            assert_eq!(
                &expected_sec_url_pattern, sec_url,
                "Original SEC filing URL is incorrectly formatted"
            )
        }

        // Test filtering by form type (10-K filings only)
        let form_opts = FilingOptions::new().with_form_type("10-K").with_limit(2);
        let form_filing_links = edgar
            .get_text_filing_links("320193", Some(form_opts))
            .await
            .unwrap();

        // Verify all returned filings are 10-K forms
        for (filing, _, _) in &form_filing_links {
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
    async fn test_sgml_header_url_format() {
        // Create Edgar instance
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        // Test specific URL pattern with well-known CIK and accession number
        let cik = "1889983"; // Example CIK
        let accession_number = "0001213900-23-009668"; // Example accession number

        let url = edgar.get_sgml_header_url(cik, accession_number).unwrap();

        // Expected URL pattern
        let formatted_acc = accession_number.replace("-", "");
        let expected_url = format!(
            "{}/data/{}/{}/{}.hdr.sgml",
            edgar.edgar_archives_url, cik, formatted_acc, accession_number
        );

        assert_eq!(url, expected_url);
    }

    #[tokio::test]
    async fn test_get_sgml_header_links() {
        // Create Edgar instance
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        // Test with Apple Inc. CIK and limit to 3 filings
        let opts = FilingOptions::new().with_limit(3);
        let filing_links = edgar
            .get_sgml_header_links("320193", Some(opts))
            .await
            .unwrap();

        // Verify we got the right number of links
        assert_eq!(
            filing_links.len(),
            3,
            "Should return exactly 3 filing links"
        );

        // Verify each link is properly formatted for SGML headers
        for (filing, url, _) in &filing_links {
            // URL should contain the .hdr.sgml extension
            assert!(url.ends_with(".hdr.sgml"), "URL should end with .hdr.sgml");

            // Verify format: {base}/data/{cik}/{acc_no_without_dashes}/{acc_no_with_dashes}.hdr.sgml
            let formatted_acc = filing.accession_number.replace("-", "");
            assert!(
                url.contains(&formatted_acc),
                "URL should contain the accession number without dashes"
            );
            assert!(
                url.contains(&filing.accession_number),
                "URL should contain the original accession number"
            );
        }
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
