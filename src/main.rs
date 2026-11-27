mod app;
mod bluetooth;
mod config;
mod input;
mod theme;
mod ui;

use std::time::Duration;

use anyhow::Result;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{app::App, bluetooth::Update, config::Config};

const TICK_RATE: Duration = Duration::from_millis(100);

const HELP: &str = "\
bluey - a keyboard-driven Bluetooth manager

Usage:
    bluey [options]

Options:
    -h, --help       Print this help
    -V, --version    Print version

Press '?' inside the application for key bindings.";

#[tokio::main]
async fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("-h" | "--help") => {
            println!("{HELP}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("bluey {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(other) => {
            eprintln!("bluey: unknown option '{other}'\n\n{HELP}");
            std::process::exit(2);
        }
        None => {}
    }

    // Connect to BlueZ *before* taking over the terminal so that setup errors
    // are printed as ordinary, readable messages.
    let config = Config::load();
    let (handle, updates) = match bluetooth::spawn().await {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("bluey: {err:#}");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, App::new(handle, config), updates).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut DefaultTerminal,
    mut app: App,
    mut updates: UnboundedReceiver<Update>,
) -> Result<()> {
    let mut input = input::spawn();
    let mut tick = tokio::time::interval(TICK_RATE);

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        tokio::select! {
            Some(event) = input.recv() => app.on_input(event),
            Some(update) = updates.recv() => app.on_update(update),
            _ = tick.tick() => app.on_tick(),
        }
    }
    Ok(())
}
