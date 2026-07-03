mod app;
mod net;
mod ui;

use std::io::stdout;
use std::panic;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use meilisearch_sdk::client::Client;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{self, UnboundedSender};

use app::{apply_event, AppEvent, Mode, Model, Screen, PAGE_SIZE};

#[derive(Parser)]
#[command(about = "A TUI to manage Meilisearch indexes")]
struct Cli {
    /// Meilisearch host URL
    #[arg(long, env = "MEILI_URL", default_value = "http://localhost:7700")]
    url: String,
    /// Meilisearch API / master key
    #[arg(long, env = "MEILI_MASTER_KEY")]
    key: Option<String>,
}

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(200);

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let url = cli.url.clone();
    let key_opt = cli.key.clone();
    let client = Client::new(cli.url, cli.key)?;

    install_panic_hook();
    let mut terminal = init_terminal()?;

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let mut model = Model::new();
    net::load_indexes(url.clone(), key_opt.clone(), tx.clone());

    // Debounce bookkeeping for search-as-you-type.
    let mut pending_search: Option<Instant> = None;

    while model.running {
        terminal.draw(|f| ui::view(&mut model, f))?;

        // Drain any async results without blocking.
        while let Ok(ev) = rx.try_recv() {
            apply_event(&mut model, ev);
        }

        // Fire a debounced search once the user pauses typing.
        if let Some(at) = pending_search {
            if at.elapsed() >= SEARCH_DEBOUNCE {
                pending_search = None;
                fire_documents(&client, &mut model, &tx);
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(
                        key.code,
                        &client,
                        &url,
                        &key_opt,
                        &mut model,
                        &tx,
                        &mut pending_search,
                    );
                }
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

/// Fire the correct documents request (browse vs search) with a fresh seq.
fn fire_documents(client: &Client, model: &mut Model, tx: &UnboundedSender<AppEvent>) {
    model.query_seq += 1;
    let seq = model.query_seq;
    let uid = model.current_index.clone();
    if model.query.is_empty() {
        net::browse_documents(client.clone(), uid, model.page, seq, tx.clone());
    } else {
        net::search_documents(
            client.clone(),
            uid,
            model.query.clone(),
            model.page,
            seq,
            tx.clone(),
        );
    }
}

fn handle_key(
    code: KeyCode,
    client: &Client,
    url: &str,
    key: &Option<String>,
    model: &mut Model,
    tx: &UnboundedSender<AppEvent>,
    pending_search: &mut Option<Instant>,
) {
    match model.mode {
        Mode::CreateInput => match code {
            KeyCode::Esc => {
                model.mode = Mode::Normal;
                model.input.clear();
            }
            KeyCode::Enter => {
                let uid = model.input.trim().to_string();
                model.input.clear();
                model.mode = Mode::Normal;
                if !uid.is_empty() {
                    net::create_index(
                        client.clone(),
                        url.to_string(),
                        key.clone(),
                        uid,
                        tx.clone(),
                    );
                }
            }
            KeyCode::Backspace => {
                model.input.pop();
            }
            KeyCode::Char(c) => model.input.push(c),
            _ => {}
        },
        Mode::SearchInput => match code {
            KeyCode::Esc => {
                model.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                model.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                model.input.pop();
                model.query = model.input.clone();
                model.page = 0;
                *pending_search = Some(Instant::now());
            }
            KeyCode::Char(c) => {
                model.input.push(c);
                model.query = model.input.clone();
                model.page = 0;
                *pending_search = Some(Instant::now());
            }
            _ => {}
        },
        Mode::Normal => match model.screen {
            Screen::IndexList => handle_index_key(code, client, url, key, model, tx),
            Screen::Documents => handle_docs_key(code, client, model, tx),
        },
    }
}

fn handle_index_key(
    code: KeyCode,
    client: &Client,
    url: &str,
    key: &Option<String>,
    model: &mut Model,
    tx: &UnboundedSender<AppEvent>,
) {
    match code {
        KeyCode::Char('q') => model.running = false,
        KeyCode::Char('r') => net::load_indexes(url.to_string(), key.clone(), tx.clone()),
        KeyCode::Char('c') => {
            model.mode = Mode::CreateInput;
            model.input.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            model.index_sel = model.index_sel.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if model.index_sel + 1 < model.indexes.len() {
                model.index_sel += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(row) = model.selected_index() {
                model.current_index = row.uid.clone();
                model.screen = Screen::Documents;
                model.query.clear();
                model.input.clear();
                model.page = 0;
                model.doc_sel = 0;
                fire_documents(client, model, tx);
            }
        }
        _ => {}
    }
}

fn handle_docs_key(
    code: KeyCode,
    client: &Client,
    model: &mut Model,
    tx: &UnboundedSender<AppEvent>,
) {
    match code {
        KeyCode::Esc => {
            model.screen = Screen::IndexList;
        }
        KeyCode::Char('q') => model.running = false,
        KeyCode::Char('/') => {
            model.mode = Mode::SearchInput;
            model.input = model.query.clone();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            model.doc_sel = model.doc_sel.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if model.doc_sel + 1 < model.docs.len() {
                model.doc_sel += 1;
            }
        }
        KeyCode::Char('n') => {
            let pages = model.total_hits.div_ceil(PAGE_SIZE).max(1);
            if model.page + 1 < pages {
                model.page += 1;
                model.doc_sel = 0;
                fire_documents(client, model, tx);
            }
        }
        KeyCode::Char('p') if model.page > 0 => {
            model.page -= 1;
            model.doc_sel = 0;
            fire_documents(client, model, tx);
        }
        _ => {}
    }
}

fn init_terminal() -> color_eyre::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
}

fn restore_terminal() -> color_eyre::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn install_panic_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        hook(info);
    }));
}
