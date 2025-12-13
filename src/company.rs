//! Company metadata and XBRL endpoints.
//!
//! This module covers two broad sets of SEC-provided data:
//! - Company identity lookups (ticker ↔ CIK) used to bootstrap most EDGAR requests.
//! - XBRL “company facts”, per-concept series, and cross-company “frames” for building
//!   time series and comparable datasets.
//!
//! Most users will start with `company_cik("AAPL")` to resolve a ticker into a CIK,
//! then call `company_facts(cik)` or `company_concept(cik, taxonomy, tag)` depending
//! on whether they need the full dataset or a targeted slice.

use super::CompanyOperations;
use super::Edgar;
use super::error::{EdgarError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

/// Mapping between stock ticker symbols and company CIKs.
///
/// This structure represents a company's stock ticker along with its Central Index Key
/// (CIK) and official title. The SEC maintains this mapping to help users discover
/// company identifiers for EDGAR queries. Note that companies can have multiple tickers
/// across different exchanges.
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyTicker {
    #[serde(rename = "cik_str")]
    pub cik: u64,
    pub ticker: String,
    pub title: String,
}

/// Mutual fund ticker with series and class identifiers.
///
/// Mutual funds have a more complex structure than regular companies, with series
/// and class designations. This struct provides the full identification information
/// needed to uniquely identify a specific mutual fund share class.
#[derive(Debug, Deserialize)]
pub struct MutualFundTicker {
    pub cik: u64,
    pub series_id: String,
    pub class_id: String,
    pub symbol: String,
}

/// Company ticker with exchange information included.
///
/// Extends the basic ticker mapping with stock exchange details. This is useful when
/// you need to distinguish between the same company ticker listed on different exchanges
/// or when building trading or market data applications.
#[derive(Debug, Deserialize)]
pub struct CompanyTickerExchange {
    pub cik: u64,
    pub ticker: String,
    pub name: String,
    pub exchange: String,
}

/// Complete set of XBRL facts reported by a company across all filings.
///
/// This structure contains all the structured financial data that a company has reported
/// in XBRL format. Facts are organized by taxonomy (US-GAAP, DEI) and then by concept tag.
/// Each fact includes multiple data points representing different time periods, fiscal years,
/// and filings.
///
/// Use this structure when you need comprehensive historical data for a company across
/// multiple concepts and time periods. For a single concept, consider using `CompanyConcept`
/// which is more focused and lightweight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyFacts {
    pub cik: u64,
    #[serde(rename = "entityName")]
    pub entity_name: String,
    #[serde(rename = "facts")]
    pub taxonomies: TaxonomyGroups,
}

/// Container for facts grouped by taxonomy standard.
///
/// The SEC's XBRL data uses different taxonomies for different types of information.
/// US-GAAP (Generally Accepted Accounting Principles) contains financial statement data,
/// while DEI (Document and Entity Information) contains metadata about the company and filing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyGroups {
    #[serde(rename = "us-gaap")]
    pub us_gaap: HashMap<String, Fact>,
    pub dei: HashMap<String, Fact>,
}

/// A single XBRL concept with its data points across different units of measure.
///
/// Represents a specific financial or business concept (like "Revenue" or "Assets") with
/// all its reported values. The same concept may be reported in different units (USD, shares,
/// etc.), so data points are grouped by unit. Labels and descriptions help interpret what
/// the concept represents in human-readable terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub units: HashMap<String, Vec<DataPoint>>,
}

/// A single data point representing a reported value for a specific time period.
///
/// Each data point captures one instance of a reported fact, including the value, the
/// time period it covers, the filing it came from, and fiscal period information. Some
/// data points are instantaneous (balance sheet items) while others span a period (income
/// statement items), which is reflected in the optional `start` field.
///
/// The `val` field can contain either a number or a string, as some XBRL concepts are
/// non-numeric (like descriptive text fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    pub end: String,
    pub val: serde_json::Value, // Can be number or string
    pub accn: String,
    #[serde(default)]
    pub fy: Option<i32>,
    #[serde(default)]
    pub fp: Option<String>,
    pub form: String,
    pub filed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<String>,
}

/// Historical data for a single XBRL concept across a company's filings.
///
/// Similar to a `Fact` from `CompanyFacts`, but retrieved individually for targeted
/// queries. Use this when you're interested in a specific concept (like "Revenue" or
/// "Cash") and don't need the full fact set. This is more efficient for single-concept
/// analysis or time-series construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConcept {
    pub cik: u64,
    pub taxonomy: String,
    pub tag: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub units: HashMap<String, Vec<DataPoint>>,
}

/// Aggregated data for a specific concept across all companies for a time period.
///
/// Frames provide a "cross-sectional" view of XBRL data - instead of one company over
/// time, you get all companies at a specific point in time. This is useful for peer
/// comparisons, industry analysis, or building datasets of comparable companies.
///
/// For example, you could retrieve revenue for all companies for Q1 2024, enabling
/// comparative analysis and rankings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub ccp: String,
    pub tag: String,
    pub taxonomy: String,
    pub uom: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub pts: u64,
    #[serde(rename = "data")]
    pub data_points: Vec<FrameDataPoint>,
}

/// Data point for a single company within a frame aggregation.
///
/// Represents one company's reported value for the concept and time period specified
/// in the parent `Frame`. Includes entity identification, location, the actual value,
/// and a reference to the source filing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDataPoint {
    #[serde(rename = "entityName")]
    pub entity_name: String,
    pub cik: u64,
    pub val: u64,
    pub accn: String,
    pub loc: String,
    pub end: String,
}

#[derive(Debug)]
enum CompanyUrlType {
    CompanyTickers,
    CompanyTickersExchange,
    MutualFundTickers,
    CompanyFacts,
    CompanyConcept,
    Frames,
}

/// Builds EDGAR API URLs for company/XBRL endpoints.
///
/// This is an internal helper that centralizes URL formatting for the public
/// `CompanyOperations` methods. It is not part of the public API surface; prefer
/// `company_tickers`, `company_facts`, `company_concept`, and `frames`.
impl Edgar {
    fn build_company_url(&self, url_type: CompanyUrlType, params: &[&str]) -> Result<String> {
        match url_type {
            CompanyUrlType::CompanyTickers => {
                Ok(format!("{}/company_tickers.json", self.edgar_files_url))
            }
            CompanyUrlType::CompanyTickersExchange => Ok(format!(
                "{}/company_tickers_exchange.json",
                self.edgar_files_url
            )),
            CompanyUrlType::MutualFundTickers => {
                Ok(format!("{}/company_tickers_mf.json", self.edgar_files_url))
            }
            CompanyUrlType::CompanyFacts => {
                let padded_cik = format!("{:0>10}", params[0]);
                Ok(format!(
                    "{}/api/xbrl/companyfacts/CIK{}.json",
                    self.edgar_data_url, padded_cik
                ))
            }
            CompanyUrlType::CompanyConcept => {
                let (cik, taxonomy, tag) = (params[0], params[1], params[2]);
                let padded_cik = format!("{:0>10}", cik);
                Ok(format!(
                    "{}/api/xbrl/companyconcept/CIK{}/{}/{}.json",
                    self.edgar_data_url, padded_cik, taxonomy, tag
                ))
            }
            CompanyUrlType::Frames => {
                let (taxonomy, tag, unit, period) = (params[0], params[1], params[2], params[3]);
                Ok(format!(
                    "{}/api/xbrl/frames/{}/{}/{}/{}.json",
                    self.edgar_data_url, taxonomy, tag, unit, period
                ))
            }
        }
    }
}

/// A trait for parsing JSON content into a collection of structured data.
trait JsonParser {
    fn parse_json_array<T, F>(
        &self,
        content: &str,
        required_fields: &[&str],
        mapper: F,
    ) -> Result<Vec<T>>
    where
        F: Fn(&FieldExtractor, &[serde_json::Value]) -> Option<T>;
}

/// Parses tabular SEC JSON content into a vector using field extraction and mapping.
///
/// Several SEC endpoints represent data as `{ "fields": [...], "data": [...] }` where each row in
/// `data` is positional. This helper provides a small adapter layer to map those rows into strongly
/// typed structs.
///
/// # Arguments
///
/// * `content` - A string slice containing JSON data to parse
/// * `required_fields` - A slice of string slices specifying the required field names
/// * `mapper` - A function that takes a `FieldExtractor` and array of JSON values and returns an optional value of type `T`
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<T>` if parsing is successful, or an error if:
/// * JSON parsing fails
/// * Required 'fields' or 'data' arrays are missing
/// * Field extraction fails
///
/// # Type Parameters
///
/// * `T` - The type of elements in the resulting vector
/// * `F` - The type of the mapper function
///
/// This is an internal helper used by `CompanyOperations`.
impl JsonParser for Edgar {
    fn parse_json_array<T, F>(
        &self,
        content: &str,
        required_fields: &[&str],
        mapper: F,
    ) -> Result<Vec<T>>
    where
        F: Fn(&FieldExtractor, &[serde_json::Value]) -> Option<T>,
    {
        let json: serde_json::Value = serde_json::from_str(content)?;

        let fields = json["fields"]
            .as_array()
            .ok_or_else(|| EdgarError::InvalidResponse("Missing 'fields' array".to_string()))?;

        let data = json["data"]
            .as_array()
            .ok_or_else(|| EdgarError::InvalidResponse("Missing 'data' array".to_string()))?;

        let extractor = FieldExtractor::new(fields.to_vec(), required_fields)?;

        Ok(data
            .iter()
            .filter_map(|row| row.as_array().and_then(|r| mapper(&extractor, r)))
            .collect())
    }
}

struct FieldExtractor {
    indices: HashMap<String, usize>,
}

/// A utility struct for extracting fields from rows of tabular SEC JSON data.
///
/// This struct helps manage field extraction by maintaining a mapping between field names
/// and their positions in data rows.
///
/// # Fields
///
/// * `indices` - HashMap storing the mapping between field names and their indices
///
/// This is an internal helper used by `CompanyOperations`.
impl FieldExtractor {
    /// Creates a new `FieldExtractor` from a vector of fields and a slice of required field names.
    ///
    /// # Arguments
    ///
    /// * `fields` - Vector of JSON values representing available fields
    /// * `required` - Slice of field names that must be present
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - New FieldExtractor instance or error if required fields are missing
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::InvalidResponse` if any required field is not found in the fields vector
    fn new(fields: Vec<serde_json::Value>, required: &[&str]) -> Result<Self> {
        let mut indices = HashMap::new();

        for field_name in required {
            let idx = fields
                .iter()
                .position(|field| field.as_str() == Some(field_name))
                .ok_or_else(|| {
                    EdgarError::InvalidResponse(format!("Missing '{}' field", field_name))
                })?;
            indices.insert(field_name.to_string(), idx);
        }

        Ok(Self { indices })
    }

    /// Gets the index for a given field name.
    ///
    /// # Arguments
    ///
    /// * `field` - Name of the field to look up
    ///
    /// # Returns
    ///
    /// * `Result<usize>` - Index of the field or error if field is not found
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::InvalidResponse` if the field is not found
    fn get_index(&self, field: &str) -> Result<usize> {
        self.indices
            .get(field)
            .copied()
            .ok_or_else(|| EdgarError::InvalidResponse(format!("Field '{}' not found", field)))
    }

    /// Extracts a value from a row of JSON data using a provided converter function.
    ///
    /// # Arguments
    ///
    /// * `row` - Slice of JSON values representing a data row
    /// * `field` - Name of the field to extract
    /// * `converter` - Function to convert the JSON value to the desired type
    ///
    /// # Returns
    ///
    /// * `Option<T>` - Converted value if field exists and conversion succeeds, None otherwise
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type for conversion
    /// * `F` - Type of the converter function
    fn extract_value<T, F>(&self, row: &[serde_json::Value], field: &str, converter: F) -> Option<T>
    where
        F: Fn(&serde_json::Value) -> Option<T>,
    {
        let idx = self.get_index(field).ok()?;
        row.get(idx).and_then(converter)
    }
}

/// Implementation of company-related operations for the SEC EDGAR database.
///
/// This implementation provides methods to interact with various company-related endpoints
/// of the SEC EDGAR database, including retrieving company tickers, CIK numbers,
/// mutual fund information, company facts, and specific financial concepts.
///
/// # Methods Overview
///
/// - `company_tickers`: Fetches a list of all company tickers and their information
/// - `company_cik`: Retrieves a CIK number for a specific company ticker
/// - `mutual_fund_cik`: Retrieves a CIK number for a specific mutual fund ticker
/// - `company_tickers_with_exchange`: Fetches company tickers with their exchange information
/// - `mutual_fund_tickers`: Retrieves a list of mutual fund tickers and their information
/// - `company_facts`: Fetches company facts for a specific CIK
/// - `company_concept`: Retrieves specific concept data for a company
/// - `frames`: Fetches standardized data frames for financial concepts
///
/// # Examples
///
/// ```ignore
/// # use edgarkit::{Edgar, CompanyOperations};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let edgar = Edgar::new("MyApp contact@example.com")?;
///
/// // Get CIK for a company
/// let apple_cik = edgar.company_cik("AAPL").await?;
///
/// // Get company facts
/// let facts = edgar.company_facts(apple_cik).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Methods in this implementation may return various error types wrapped in `EdgarError`:
/// - Network-related errors during HTTP requests
/// - Parse errors for malformed JSON responses
/// - Not found errors for invalid tickers or CIKs
/// - Invalid response errors for unexpected API responses
#[async_trait]
impl CompanyOperations for Edgar {
    /// Retrieves a list of company tickers from the SEC EDGAR database.
    ///
    /// This function fetches the company_tickers.json file from the SEC EDGAR database,
    /// which contains information about company tickers, CIK numbers, and company names.
    /// It then parses this data into a vector of `CompanyTicker` structs.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<CompanyTicker>>` - On success, returns `Ok` containing a vector of `CompanyTicker` structs.
    ///   Each `CompanyTicker` contains information about a single company.
    ///   On failure, returns `Err` containing an `EdgarError` indicating the type of error that occurred.
    ///
    /// # Errors
    ///
    /// * `EdgarError::RequestError` - If there was an error sending the request or reading the response.
    /// * `EdgarError::NotFound` - If the company_tickers.json file was not found.
    /// * `EdgarError::InvalidResponse` - If the response couldn't be parsed as expected.
    async fn company_tickers(&self) -> Result<Vec<CompanyTicker>> {
        let url = self.build_company_url(CompanyUrlType::CompanyTickers, &[])?;
        let response = self.get(&url).await?;
        let map: HashMap<String, CompanyTicker> = serde_json::from_str(&response)?;
        Ok(map.into_values().collect())
    }

    /// Retrieves the Central Index Key (CIK) for a given company ticker symbol.
    ///
    /// This function searches for a company's CIK using its ticker symbol. It first fetches
    /// all company tickers and then finds the matching ticker, returning its associated CIK.
    ///
    /// # Arguments
    ///
    /// * `ticker` - A string slice that holds the ticker symbol of the company.
    ///
    /// # Returns
    ///
    /// * `Result<u64>` - On success, returns `Ok` containing the CIK as a u64.
    ///   On failure, returns `Err` containing an `EdgarError`.
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::TickerNotFound` if the provided ticker symbol is not found.
    async fn company_cik(&self, ticker: &str) -> Result<u64> {
        let tickers = self.company_tickers().await?;

        let company = tickers
            .iter()
            .find(|t| t.ticker == ticker.to_uppercase())
            .ok_or(EdgarError::TickerNotFound)?;

        Ok(company.cik.clone())
    }

    /// Retrieves the Central Index Key (CIK) for a given mutual fund ticker symbol.
    ///
    /// This function searches for a mutual fund's CIK using its ticker symbol. It first fetches
    /// all mutual fund tickers and then finds the matching ticker, returning its associated CIK.
    ///
    /// # Arguments
    ///
    /// * `ticker` - A string slice that holds the ticker symbol of the mutual fund.
    ///
    /// # Returns
    ///
    /// * `Result<u64>` - On success, returns `Ok` containing the CIK as a u64.
    ///   On failure, returns `Err` containing an `EdgarError`.
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::TickerNotFound` if the provided ticker symbol is not found.
    async fn mutual_fund_cik(&self, ticker: &str) -> Result<u64> {
        let tickers = self.mutual_fund_tickers().await?;

        let fund = tickers
            .iter()
            .find(|t| t.symbol == ticker.to_uppercase())
            .ok_or(EdgarError::TickerNotFound)?;

        Ok(fund.cik.clone())
    }

    /// Retrieves a list of company tickers with their associated exchange information from the SEC EDGAR database.
    ///
    /// This function fetches the company_tickers_exchange.json file from the SEC EDGAR database,
    /// which contains information about company tickers, CIK numbers, company names, and their associated exchanges.
    /// It then parses this data into a vector of `CompanyTickerExchange` structs.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<CompanyTickerExchange>>` - On success, returns `Ok` containing a vector of `CompanyTickerExchange` structs.
    ///   Each `CompanyTickerExchange` contains information about a single company, including its CIK, name, ticker, and exchange.
    ///   On failure, returns `Err` containing an `EdgarError` indicating the type of error that occurred.
    ///
    /// # Errors
    ///
    /// This function may return an error in the following cases:
    /// * If there's a network error while fetching the data.
    /// * If the response from the SEC EDGAR database is invalid or cannot be parsed.
    /// * If the required fields are missing from the response.
    async fn company_tickers_with_exchange(&self) -> Result<Vec<CompanyTickerExchange>> {
        let url = self.build_company_url(CompanyUrlType::CompanyTickersExchange, &[])?;
        let response = self.get(&url).await?;

        self.parse_json_array(
            &response,
            &["cik", "name", "ticker", "exchange"],
            |extractor, row| {
                Some(CompanyTickerExchange {
                    cik: extractor.extract_value(row, "cik", |v| v.as_str()?.parse().ok())?,
                    name: extractor.extract_value(row, "name", |v| v.as_str().map(String::from))?,
                    ticker: extractor
                        .extract_value(row, "ticker", |v| v.as_str().map(String::from))?,
                    exchange: extractor
                        .extract_value(row, "exchange", |v| v.as_str().map(String::from))?,
                })
            },
        )
    }

    /// Retrieves a list of mutual fund tickers from the SEC EDGAR database.
    ///
    /// This function fetches the company_tickers_mf.json file from the SEC EDGAR database,
    /// which contains information about mutual fund tickers, CIK numbers, and fund names.
    /// It then parses this data into a vector of `MutualFundTicker` structs.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<MutualFundTicker>>` - On success, returns `Ok` containing a vector of `MutualFundTicker` structs.
    ///   Each `MutualFundTicker` contains information about a single mutual fund.
    ///   On failure, returns `Err` containing an `EdgarError` indicating the type of error that occurred.
    ///
    /// # Errors
    ///
    /// * `EdgarError::RequestError` - If there was an error sending the request or reading the response.
    /// * `EdgarError::NotFound` - If the company_tickers_mf.json file was not found.
    /// * `EdgarError::InvalidResponse` - If the response couldn't be parsed as expected.
    async fn mutual_fund_tickers(&self) -> Result<Vec<MutualFundTicker>> {
        let url = self.build_company_url(CompanyUrlType::MutualFundTickers, &[])?;
        let response = self.get(&url).await?;

        self.parse_json_array(
            &response,
            &["cik", "seriesId", "classId", "symbol"],
            |extractor, row| {
                Some(MutualFundTicker {
                    cik: extractor.extract_value(row, "cik", |v| v.as_u64())?,
                    series_id: extractor
                        .extract_value(row, "seriesId", |v| v.as_str().map(String::from))?,
                    class_id: extractor
                        .extract_value(row, "classId", |v| v.as_str().map(String::from))?,
                    symbol: extractor
                        .extract_value(row, "symbol", |v| v.as_str().map(String::from))?,
                })
            },
        )
    }

    /// Retrieves company facts for a specific company identified by its Central Index Key (CIK).
    ///
    /// This function fetches comprehensive financial and operational data about a company
    /// from the SEC EDGAR database. The data includes various financial metrics, operational
    /// statistics, and other relevant information filed by the company.
    ///
    /// # Arguments
    ///
    /// * `cik` - A 64-bit unsigned integer representing the Central Index Key (CIK) of the company.
    ///           The CIK is a unique identifier assigned by the SEC to each entity that files reports.
    ///
    /// # Returns
    ///
    /// * `Result<CompanyFacts>` - On success, returns `Ok` containing a `CompanyFacts` struct
    ///   which encapsulates all the retrieved facts about the company. On failure, returns
    ///   an `Err` containing an `EdgarError` describing what went wrong.
    ///
    /// # Errors
    ///
    /// This function may return an error if:
    /// * There's a network issue while fetching the data
    /// * The SEC EDGAR API returns an unexpected response
    /// * The response cannot be parsed into the `CompanyFacts` structure
    async fn company_facts(&self, cik: u64) -> Result<CompanyFacts> {
        let url = self.build_company_url(CompanyUrlType::CompanyFacts, &[&cik.to_string()])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str(&response)?)
    }

    /// Fetches and parses company-specific financial concepts for a given company identified by its Central Index Key (CIK).
    ///
    /// # Parameters
    ///
    /// * `cik` - A 64-bit unsigned integer representing the CIK of the company.
    /// * `taxonomy` - A string representing the financial taxonomy, such as "us-gaap" or "ifrs".
    /// * `tag` - A string representing the specific financial concept within the taxonomy.
    ///
    /// # Returns
    ///
    /// * `Result<CompanyConcept>` - On success, returns a `CompanyConcept` struct containing the parsed financial concept data.
    ///   On failure, returns an `Err` containing an `EdgarError` describing what went wrong.
    async fn company_concept(&self, cik: u64, taxonomy: &str, tag: &str) -> Result<CompanyConcept> {
        let url = self.build_company_url(
            CompanyUrlType::CompanyConcept,
            &[&cik.to_string(), taxonomy, tag],
        )?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str(&response)?)
    }

    /// Fetches and parses financial frames for a specific company identified by its Central Index Key (CIK)
    /// and financial concept within a given taxonomy, unit, and period.
    ///
    /// # Parameters
    ///
    /// * `taxonomy` - A string representing the financial taxonomy, such as "us-gaap" or "ifrs".
    /// * `tag` - A string representing the specific financial concept within the taxonomy.
    /// * `unit` - A string representing the unit of measurement for the financial concept, such as "USD" or "EUR".
    /// * `period` - A string representing the financial period, such as "CY2019Q1I" or "FY2020".
    ///
    /// # Returns
    ///
    /// * `Result<Frame>` - On success, returns a `Result` containing a `Frame` struct representing the parsed financial frames.
    ///   On failure, returns an `Err` containing an `EdgarError` describing what went wrong.
    async fn frames(&self, taxonomy: &str, tag: &str, unit: &str, period: &str) -> Result<Frame> {
        let url = self.build_company_url(CompanyUrlType::Frames, &[taxonomy, tag, unit, period])?;
        let response = self.get(&url).await?;
        Ok(serde_json::from_str(&response)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const COMPANY_FACTS_FIXTURE: &str = "../fixtures/tickers/companyfacts.json";
    const COMPANY_CONCEPT_FIXTURE: &str = "../fixtures/tickers/companyconcept.json";
    const COMPANY_TICKERS_EXCHANGE_FIXTURE: &str =
        "../fixtures/tickers/company_tickers_exchange.json";
    const MUTUAL_FUND_TICKERS_FIXTURE: &str = "../fixtures/tickers/company_tickers_mf.json";

    #[test]
    fn test_parse_company_tickers_exchange() {
        let content = fs::read_to_string(COMPANY_TICKERS_EXCHANGE_FIXTURE).unwrap();
        let edgar = Edgar::new("test_agent").unwrap();

        let tickers = edgar
            .parse_json_array(
                &content,
                &["cik", "name", "ticker", "exchange"],
                |extractor, row| {
                    Some(CompanyTickerExchange {
                        cik: extractor.extract_value(row, "cik", |v| v.as_u64())?,
                        name: extractor
                            .extract_value(row, "name", |v| v.as_str().map(String::from))?,
                        ticker: extractor
                            .extract_value(row, "ticker", |v| v.as_str().map(String::from))?,
                        exchange: extractor
                            .extract_value(row, "exchange", |v| v.as_str().map(String::from))?,
                    })
                },
            )
            .unwrap();

        assert_eq!(tickers[0].cik, 320193);
        assert_eq!(tickers[0].name, "Apple Inc.");
        assert_eq!(tickers[0].ticker, "AAPL");
        assert_eq!(tickers[0].exchange, "Nasdaq");

        assert_eq!(tickers[1].cik, 1045810);
        assert_eq!(tickers[1].name, "NVIDIA CORP");
        assert_eq!(tickers[1].ticker, "NVDA");
        assert_eq!(tickers[1].exchange, "Nasdaq");
    }

    #[tokio::test]
    async fn test_company_cik() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let cik = edgar.company_cik("AAPL").await.unwrap();
        assert_eq!(cik, 320193);
    }

    #[tokio::test]
    async fn test_company_cik_not_found() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.company_cik("INVALID").await;
        assert!(matches!(result, Err(EdgarError::TickerNotFound)));
    }

    #[test]
    fn test_parse_mutual_fund_tickers() {
        let content = fs::read_to_string(MUTUAL_FUND_TICKERS_FIXTURE).unwrap();
        let edgar = Edgar::new("test_agent").unwrap();

        let tickers = edgar
            .parse_json_array(
                &content,
                &["cik", "seriesId", "classId", "symbol"],
                |extractor, row| {
                    Some(MutualFundTicker {
                        cik: extractor.extract_value(row, "cik", |v| v.as_u64())?,
                        series_id: extractor
                            .extract_value(row, "seriesId", |v| v.as_str().map(String::from))?,
                        class_id: extractor
                            .extract_value(row, "classId", |v| v.as_str().map(String::from))?,
                        symbol: extractor
                            .extract_value(row, "symbol", |v| v.as_str().map(String::from))?,
                    })
                },
            )
            .unwrap();

        assert_eq!(tickers[0].cik, 2110);
        assert_eq!(tickers[0].series_id, "S000009184");
        assert_eq!(tickers[0].class_id, "C000024954");
        assert_eq!(tickers[0].symbol, "LACAX");
    }

    #[tokio::test]
    async fn test_mutual_fund_cik() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let cik = edgar.mutual_fund_cik("LACAX").await.unwrap();
        assert_eq!(cik, 2110);
    }

    #[tokio::test]
    async fn test_mutual_fund_cik_not_found() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.mutual_fund_cik("INVALID").await;
        assert!(matches!(result, Err(EdgarError::TickerNotFound)));
    }

    #[test]
    fn test_parse_invalid_json() {
        let edgar = Edgar::new("test_agent").unwrap();
        let result =
            edgar.parse_json_array::<CompanyTicker, _>("invalid json", &["cik"], |_, _| None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_fact_with_null_fields() {
        let json = r#"{
                "label": null,
                "description": null,
                "units": {
                    "USD": [
                        {
                            "end": "2021-12-31",
                            "val": 1000000,
                            "accn": "0001234567-21-000001",
                            "fy": 2021,
                            "fp": "FY",
                            "form": "10-K",
                            "filed": "2022-01-31"
                        }
                    ]
                }
            }"#;

        let fact: Fact = serde_json::from_str(json).unwrap();
        assert!(fact.label.is_none());
        assert!(fact.description.is_none());
        assert!(!fact.units.is_empty());
    }

    #[test]
    fn test_parse_company_facts() {
        let content = fs::read_to_string(COMPANY_FACTS_FIXTURE).unwrap();
        let facts: CompanyFacts = serde_json::from_str(&content).unwrap();

        assert_eq!(facts.cik, 320193);
        assert_eq!(facts.entity_name, "Apple Inc.");

        // Test US-GAAP fact
        let income_tax = facts
            .taxonomies
            .us_gaap
            .get("IncomeTaxExpenseBenefit")
            .unwrap();
        assert_eq!(
            income_tax.label,
            Some("Income Tax Expense (Benefit)".to_string())
        );

        // Test data point
        let data_points = income_tax.units.get("USD").unwrap();
        let point = &data_points[0];
        assert_eq!(point.val, 1512000000);
        assert_eq!(point.form, "10-K");
        assert_eq!(point.filed, "2009-10-27");
        assert!(point.frame.is_none());
    }

    #[tokio::test]
    async fn test_company_facts_not_found() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.company_facts(0).await;
        assert!(matches!(result, Err(EdgarError::NotFound)));
    }

    #[test]
    fn test_parse_company_concept() {
        let content = fs::read_to_string(COMPANY_CONCEPT_FIXTURE).unwrap();
        let concept: CompanyConcept = serde_json::from_str(&content).unwrap();

        assert_eq!(concept.cik, 320193);
        assert_eq!(concept.taxonomy, "dei");
        assert_eq!(concept.tag, "EntityCommonStockSharesOutstanding");
        assert!(!concept.units.is_empty());

        let data_points = concept.units.get("shares").unwrap();
        let point = &data_points[0];
        assert!(point.val.is_number());
        assert_eq!(point.form, "10-Q");
    }

    #[tokio::test]
    async fn test_company_concept() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let concept = edgar
            .company_concept(320193, "dei", "EntityCommonStockSharesOutstanding")
            .await
            .unwrap();
        assert_eq!(concept.taxonomy, "dei");
        assert_eq!(concept.tag, "EntityCommonStockSharesOutstanding");
    }

    #[test]
    fn test_parse_frames() {
        let content = fs::read_to_string("../fixtures/tickers/frames.json").unwrap();
        let frame: Frame = serde_json::from_str(&content).unwrap();

        assert_eq!(frame.taxonomy, "us-gaap");
        assert_eq!(frame.tag, "AccountsPayableCurrent");
        assert_eq!(frame.uom, "USD");
        assert_eq!(frame.ccp, "CY2019Q1I");

        // Test data points
        let point = &frame.data_points[0];
        assert_eq!(point.cik, 1750);
        assert_eq!(point.entity_name, "AAR CORP.");
        assert_eq!(point.loc, "US-IL");
        assert_eq!(point.val, 218600000);
        assert_eq!(point.accn, "0001104659-19-016320");
        assert_eq!(point.end, "2019-02-28");
    }
}
