use anyhow::{Context, Result};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use clap::{ArgGroup, Parser};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use edgarkit::{
    DetailedFiling, Edgar, EdgarDay, EdgarError, FilingOperations, FilingOptions, IndexOperations,
    Submission,
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use std::{collections::HashSet, io, time::Duration as StdDuration};

#[derive(Debug, Parser)]
#[command(name = "ipo-scanner")]
#[command(
    group(
        ArgGroup::new("window")
            .required(true)
            .args(["day", "weekly", "monthly"])
    )
)]
struct Args {
    /// Scan today's daily index.
    #[arg(long)]
    day: bool,

    /// Scan the last 7 days (inclusive).
    #[arg(long)]
    weekly: bool,

    /// Scan the last 30 days (inclusive).
    #[arg(long)]
    monthly: bool,

    /// SEC.gov-required user agent (e.g. "MyApp you@example.com").
    #[arg(long)]
    user_agent: String,
}

#[derive(Debug, Clone)]
struct IpoRow {
    company: String,
    cik: u64,
    date_filed: NaiveDate,
    when: String,
    index_html: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Companies,
    Filings,
}

#[derive(Debug, Default)]
struct SearchState {
    active: bool,
    query: String,
}

#[derive(Debug)]
struct App {
    screen: Screen,

    all_rows: Vec<IpoRow>,
    rows: Vec<IpoRow>,
    companies_state: TableState,
    search: SearchState,

    selected_company: Option<(u64, String)>,
    filings: Vec<DetailedFiling>,
    filings_state: TableState,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let edgar = Edgar::new(&args.user_agent).context("failed to create Edgar client")?;

    let today = Utc::now().date_naive();
    let (start, end) = if args.day {
        (today, today)
    } else if args.weekly {
        (today - Duration::days(6), today)
    } else {
        (today - Duration::days(29), today)
    };

    let rows = fetch_rows(&edgar, start, end).await?;
    run_tui(&edgar, rows).await?;
    Ok(())
}

async fn fetch_rows(edgar: &Edgar, start: NaiveDate, end: NaiveDate) -> Result<Vec<IpoRow>> {
    let mut rows = Vec::new();
    let mut seen = HashSet::<String>::new();

    let opts = FilingOptions::new()
        .with_form_type("S-1")
        .with_include_amendments(false);

    let mut day = start;
    while day <= end {
        let edgar_day = EdgarDay::new(day.year(), day.month(), day.day())?;
        match edgar.get_daily_filings(edgar_day, Some(opts.clone())).await {
            Ok(entries) => {
                for entry in entries {
                    // Defensive: keep only exact S-1.
                    if entry.form_type.trim() != "S-1" {
                        continue;
                    }

                    // SEC “browse company” page expects a zero-padded 10-digit CIK.
                    let index_html = format!("https://www.sec.gov/edgar/browse/?CIK={:010}", entry.cik);

                    if !seen.insert(index_html.clone()) {
                        continue;
                    }

                    let date_filed = NaiveDate::parse_from_str(&entry.date_filed, "%Y%m%d")
                        .with_context(|| format!("invalid date_filed: {}", entry.date_filed))?;

                    let when = human_age(Utc::now().date_naive(), date_filed);

                    rows.push(IpoRow {
                        company: normalize_company_name(&entry.company_name),
                        cik: entry.cik,
                        date_filed,
                        when,
                        index_html,
                    });
                }
            }
            Err(EdgarError::NotFound) => {
                // Weekends/holidays can be missing.
            }
            Err(e) => return Err(e.into()),
        }

        day += Duration::days(1);
    }

    rows.sort_by(|a, b| b.date_filed.cmp(&a.date_filed));
    Ok(rows)
}

fn human_age(today: NaiveDate, date: NaiveDate) -> String {
    let days = (today - date).num_days().max(0);
    if days == 0 {
        return "today".to_string();
    }
    if days < 14 {
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    }

    let weeks = days / 7;
    if weeks < 8 {
        return format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" });
    }

    let months = days / 30;
    format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
}

fn normalize_company_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let has_lower = trimmed.chars().any(|c| c.is_ascii_lowercase());
    let has_alpha = trimmed.chars().any(|c| c.is_ascii_alphabetic());
    if has_lower || !has_alpha {
        return trimmed.to_string();
    }

    // If it looks like shouting (all-caps), title-case words while keeping short acronyms.
    trimmed
        .split_whitespace()
        .map(|word| {
            if word.len() <= 3
                && word
                    .chars()
                    .all(|c| !c.is_ascii_alphabetic() || c.is_ascii_uppercase())
            {
                return word.to_string();
            }

            let mut chars = word.chars();
            let first = chars.next();
            let rest: String = chars.collect();

            match first {
                Some(c) if c.is_ascii_alphabetic() => {
                    format!("{}{}", c.to_ascii_uppercase(), rest.to_ascii_lowercase())
                }
                Some(c) => format!("{}{}", c, rest.to_ascii_lowercase()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn run_tui(edgar: &Edgar, rows: Vec<IpoRow>) -> Result<()> {
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("create terminal")?;

    let mut app = App {
        screen: Screen::Companies,
        all_rows: rows.clone(),
        rows,
        companies_state: TableState::default(),
        search: SearchState::default(),
        selected_company: None,
        filings: Vec::new(),
        filings_state: TableState::default(),
    };

    if !app.rows.is_empty() {
        app.companies_state.select(Some(0));
    }

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());

            match app.screen {
                Screen::Companies => {
                    let header = Row::new([
                        Cell::from("Company"),
                        Cell::from("CIK"),
                        Cell::from("When"),
                        Cell::from("Filing (browser)"),
                    ])
                    .style(Style::default().add_modifier(Modifier::BOLD));

                    let table_rows = app.rows.iter().map(|r| {
                        Row::new([
                            Cell::from(r.company.clone()),
                            Cell::from(r.cik.to_string()),
                            Cell::from(r.when.clone()),
                            Cell::from(r.index_html.clone()),
                        ])
                    });

                    let table = Table::new(
                        table_rows,
                        [
                            Constraint::Percentage(26),
                            Constraint::Length(12),
                            Constraint::Length(14),
                            Constraint::Percentage(48),
                        ],
                    )
                    .header(header)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("IPO Scanner (S-1)"),
                    )
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                    f.render_stateful_widget(table, chunks[0], &mut app.companies_state);

                    let footer = if app.search.active {
                        Line::from(format!(
                            "search: {}  |  Enter apply  Esc cancel  Backspace delete",
                            app.search.query
                        ))
                    } else {
                        Line::from("q quit  |  ↑/↓ move  |  Enter filings  |  s search")
                    };
                    f.render_widget(footer, chunks[1]);
                }
                Screen::Filings => {
                    let title = match &app.selected_company {
                        Some((cik, name)) => format!("Filings for {} ({})", name, cik),
                        None => "Filings".to_string(),
                    };

                    let header = Row::new([
                        Cell::from("Form"),
                        Cell::from("Filing date"),
                        Cell::from("Size"),
                        Cell::from("Primary doc description"),
                    ])
                    .style(Style::default().add_modifier(Modifier::BOLD));

                    let table_rows = app.filings.iter().map(|f| {
                        let desc = f
                            .primary_doc_description
                            .as_deref()
                            .unwrap_or("")
                            .to_string();
                        Row::new([
                            Cell::from(f.form.clone()),
                            Cell::from(f.filing_date.clone()),
                            Cell::from(format_bytes(f.size as i64)),
                            Cell::from(desc),
                        ])
                    });

                    let table = Table::new(
                        table_rows,
                        [
                            Constraint::Length(15),
                            Constraint::Length(12),
                            Constraint::Length(10),
                            Constraint::Percentage(68),
                        ],
                    )
                    .header(header)
                    .block(Block::default().borders(Borders::ALL).title(title))
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                    f.render_stateful_widget(table, chunks[0], &mut app.filings_state);

                    let footer = Line::from("b back  |  q quit  |  ↑/↓ move");
                    f.render_widget(footer, chunks[1]);
                }
            }
        })?;

        if event::poll(StdDuration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                handle_key(edgar, key, &mut app, &mut should_quit).await?;
            }
        }
    }

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    Ok(())
}

async fn handle_key(
    edgar: &Edgar,
    key: KeyEvent,
    app: &mut App,
    should_quit: &mut bool,
) -> Result<()> {
    match app.screen {
        Screen::Companies => {
            if app.search.active {
                match key.code {
                    KeyCode::Enter => app.search.active = false,
                    KeyCode::Esc => {
                        app.search.active = false;
                        app.search.query.clear();
                        apply_search(app);
                    }
                    KeyCode::Backspace => {
                        app.search.query.pop();
                        apply_search(app);
                    }
                    KeyCode::Char(c) => {
                        if !c.is_control() {
                            app.search.query.push(c);
                            apply_search(app);
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => *should_quit = true,
                KeyCode::Down => table_down(&mut app.companies_state, app.rows.len()),
                KeyCode::Up => table_up(&mut app.companies_state, app.rows.len()),
                KeyCode::Char('s') => {
                    app.search.active = true;
                }
                KeyCode::Enter => {
                    let Some(selected) = app.companies_state.selected() else {
                        return Ok(());
                    };
                    let Some(row) = app.rows.get(selected) else {
                        return Ok(());
                    };

                    let cik = row.cik;
                    let company = row.company.clone();

                    let filings = fetch_company_filings(edgar, cik).await?;

                    app.selected_company = Some((cik, company));
                    app.filings = filings;
                    app.filings_state = TableState::default();
                    if !app.filings.is_empty() {
                        app.filings_state.select(Some(0));
                    }
                    app.screen = Screen::Filings;
                }
                _ => {}
            }
        }
        Screen::Filings => match key.code {
            KeyCode::Char('q') => *should_quit = true,
            KeyCode::Char('b') | KeyCode::Esc => {
                app.screen = Screen::Companies;
            }
            KeyCode::Down => table_down(&mut app.filings_state, app.filings.len()),
            KeyCode::Up => table_up(&mut app.filings_state, app.filings.len()),
            _ => {}
        },
    }

    Ok(())
}

fn apply_search(app: &mut App) {
    let q = app.search.query.trim().to_ascii_lowercase();
    if q.is_empty() {
        app.rows = app.all_rows.clone();
    } else {
        app.rows = app
            .all_rows
            .iter()
            .cloned()
            .filter(|r| {
                r.company.to_ascii_lowercase().contains(&q) || r.cik.to_string().contains(&q)
            })
            .collect();
    }

    app.companies_state = TableState::default();
    if !app.rows.is_empty() {
        app.companies_state.select(Some(0));
    }
}

fn table_down(state: &mut TableState, row_count: usize) {
    if row_count == 0 {
        return;
    }
    let next = match state.selected() {
        Some(i) if i + 1 < row_count => i + 1,
        _ => 0,
    };
    state.select(Some(next));
}

fn table_up(state: &mut TableState, row_count: usize) {
    if row_count == 0 {
        return;
    }
    let next = match state.selected() {
        Some(i) if i > 0 => i - 1,
        _ => row_count.saturating_sub(1),
    };
    state.select(Some(next));
}

async fn fetch_company_filings(edgar: &Edgar, cik: u64) -> Result<Vec<DetailedFiling>> {
    let submission: Submission = edgar.submissions(&cik.to_string()).await?;
    let recent = &submission.filings.recent;

    let mut out = Vec::new();
    for idx in 0..recent.accession_number.len() {
        if let Ok(filing) = DetailedFiling::try_from((recent, idx)) {
            out.push(filing);
        }
    }

    // SEC usually returns newest-first already.
    Ok(out)
}

fn format_bytes(bytes: i64) -> String {
    if bytes < 0 {
        return "".to_string();
    }
    let b = bytes as f64;
    if b < 1024.0 {
        return format!("{} B", bytes);
    }
    let kb = b / 1024.0;
    if kb < 1024.0 {
        return format!("{:.1} KB", kb);
    }
    let mb = kb / 1024.0;
    if mb < 1024.0 {
        return format!("{:.1} MB", mb);
    }
    let gb = mb / 1024.0;
    format!("{:.1} GB", gb)
}
