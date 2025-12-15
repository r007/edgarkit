# investment-adviser

A small CLI that fetches the latest 10-Q/10-K from SEC EDGAR (by ticker or CIK) and asks an OpenRouter-hosted model for a buy/hold/avoid recommendation.

## Prereqs

- An OpenRouter API key in `OPENROUTER_API_KEY`

## Run

From `src/etl/edgar/examples/investment-adviser`:

```bash
export OPENROUTER_API_KEY="..."

cargo run --release -- \
  --ticker AAPL \
  --user-agent "YourApp you@example.com" \
  --model deepseek/deepseek-v3.2
```

Or by CIK:

```bash
export OPENROUTER_API_KEY="..."

cargo run --release -- \
  --cik 320193 \
  --user-agent "YourApp you@example.com"
```
