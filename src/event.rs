use std::time::Duration;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};

#[allow(dead_code)]
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> Result<AppEvent, anyhow::Error> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(AppEvent::Key(key)),
                Event::Mouse(mouse) => Ok(AppEvent::Mouse(mouse)),
                Event::Resize(w, h) => Ok(AppEvent::Resize(w, h)),
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}
