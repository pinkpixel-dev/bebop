use std::env;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use auri::app::App;
use auri::event::{AppEvent, EventHandler};
use auri::terminal::KittyRenderer;

fn main() -> Result<(), anyhow::Error> {
    // Install panic hook to ensure terminal is cleanly restored on error
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut out = io::stdout();
        let _ = execute!(out, LeaveAlternateScreen, DisableMouseCapture);
        let _ = KittyRenderer::clear_all();
        default_panic(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    // Check CLI argument for audio file or directory
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let target = PathBuf::from(&args[1]);
        let _ = app.open_target(&target);
    } else {
        // Look for default ~/Music folder if it exists
        if let Some(music_dir) = dirs::audio_dir() {
            if music_dir.exists() {
                let _ = app.load_directory(&music_dir);
            }
        }
    }

    let events = EventHandler::new(Duration::from_millis(30));

    // Main application loop
    while !app.should_quit {
        app.draw(&mut terminal)?;

        match events.next()? {
            AppEvent::Key(key) => app.on_key(key),
            AppEvent::Mouse(mouse) => app.on_mouse(mouse),
            AppEvent::Resize(_, _) => {
                terminal.autoresize()?;
            }
            AppEvent::Tick => app.on_tick(),
        }
    }

    // Clean terminal restoration
    let _ = KittyRenderer::clear_all();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}
