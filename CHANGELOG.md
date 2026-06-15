# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

- Removed the `cache` Cargo feature and the `moka` dependency it pulled in; `Edgar::get` no longer caches HTTP responses
- Removed `EdgarConfig::cache_ttl` and `EdgarConfig::cache_capacity` fields; construct `EdgarConfig` via `EdgarConfig::new(...)` or struct-update syntax to migrate

### Added

- `RecentFilings::is_xbrl_numeric` field (`isXBRLNumeric`) — nullable parallel array indicating XBRL numeric data presence
- `DetailedFiling::is_xbrl_numeric` field derived from the above
- `DetailedFiling::has_xbrl_data()` helper that returns `true` if the filing has any XBRL data (standard, inline, or numeric)

## [0.2.1] - 2026-05-18

### Fixed

- `Edgar::get` was accidentally scoped `pub(crate)`, preventing external callers from using it; it is now `pub` again so `edgar.get(url).await` works as before

## [0.2.0] - 2026-05-06

### Breaking Changes

- Removed `Edgar::filings(cik, opts)` and `Edgar::get_recent_filings(cik)` — use `edgar.submissions(cik).await?` followed by `submission.filings(opts)` instead
- `FilingOperations::text_filing_links` and `sgml_header_links` are now synchronous and take `&Submission` instead of a CIK string

### Added

- HTTP response cache via [`moka`](https://crates.io/crates/moka) 0.12 behind the `cache` feature flag
- `EdgarConfig::cache_ttl` and `cache_capacity` fields to configure the in-memory cache
- `Submission::filings(opts)` — synchronous iterator over submission filings with optional filtering
- `Submission::into_detailed_filings(edgar, opts)` — async method that fetches full filing details

### Docs

- Added documentation for the private `Edgar::fetch` method describing its rate-limiting gate, JSON/HTML content-type guard, 429 retry loop, and error mappings

### Changed

- `quick-xml` upgraded from ~0.37 to 0.40.1; XML text/attribute extraction now uses `xml10_content()` and `normalized_value(XmlVersion::Implicit1_0)` per the updated API
- `governor` upgraded from 0.8 to 0.10.4

## [0.1.1] - 2026-04-11

### Changed

- Updated dependencies

### Chore

- Excluded `tests/`, `.assets/`, and `.gitignore` from the published crate package

## [0.1.0] - 2025-12-15

### Added

- Initial release of `edgarkit`, an unofficial async Rust client for the SEC EDGAR system
- `search` feature: full-text and EDGAR EFTS search with pagination support
- `filings` feature: submission lookup, filing index, SGML header retrieval, XBRL frame data, and text filing link extraction
- `company` feature: company metadata, CIK lookup, tickers, and XBRL endpoints
- `feeds` feature: Atom and RSS feed parsing for recent SEC filings
- `index` feature: quarterly index file downloads (full, company, form, crawler) with gzip support
- Rate-limited HTTP client via `governor` to respect SEC fair-use policies
- Configurable `EdgarOptions` (user-agent, base URL, rate limiting)
- Comprehensive examples: basic usage, filing downloads, index operations, RSS feeds, search
- Optional mini-project examples: investment-adviser CLI and IPO scanner TUI (S-1 filings)

[0.1.1]: https://github.com/r007/edgarkit/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/r007/edgarkit/releases/tag/v0.1.0
