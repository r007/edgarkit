# ipo-scanner

Scans recent EDGAR daily indices for `S-1` filings and shows them in a small TUI table.

## Run

From `src/etl/edgar/examples/ipo-scanner`:

```bash
cargo run --release -- --weekly --user-agent "YourApp contact@you.com"
```

Pick one window:
- `--day` (today)
- `--weekly` (last 7 days)
- `--monthly` (last 30 days)

Keys:
- `q` quit
- `↑` / `↓` move selection
- `Enter` view filings for company
- `b` / `Esc` back (from filings view)
- `s` search (company list)
