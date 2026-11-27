//! Terminal input, read on a dedicated thread and forwarded to the async loop.

use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyEvent};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Key(KeyEvent),
    Resize,
}

pub fn spawn() -> UnboundedReceiver<Input> {
    let (tx, rx) = unbounded_channel();
    std::thread::spawn(move || loop {
        match event::poll(Duration::from_millis(250)) {
            Ok(true) => {
                let forwarded = match event::read() {
                    Ok(Event::Key(key)) => tx.send(Input::Key(key)),
                    Ok(Event::Resize(..)) => tx.send(Input::Resize),
                    Ok(_) => Ok(()),
                    Err(_) => break,
                };
                if forwarded.is_err() {
                    break;
                }
            }
            Ok(false) => {
                if tx.is_closed() {
                    break;
                }
            }
            Err(_) => break,
        }
    });
    rx
}
