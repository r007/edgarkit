# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
