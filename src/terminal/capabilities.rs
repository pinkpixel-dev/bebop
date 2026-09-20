use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalGraphics {
    Kitty,
    None,
}

pub struct TerminalDetector;

impl TerminalDetector {
    pub fn detect() -> TerminalGraphics {
        // Direct Kitty window indicator
        if env::var("KITTY_WINDOW_ID").is_ok() {
            return TerminalGraphics::Kitty;
        }

        // TERM check
        if let Ok(term) = env::var("TERM") {
            if term.contains("kitty") || term.contains("ghostty") {
                return TerminalGraphics::Kitty;
            }
        }

        // TERM_PROGRAM check (WezTerm, Ghostty, Kitty support Kitty protocol)
        if let Ok(prog) = env::var("TERM_PROGRAM") {
            let p = prog.to_lowercase();
            if p.contains("kitty") || p.contains("ghostty") || p.contains("wezterm") {
                return TerminalGraphics::Kitty;
            }
        }

        TerminalGraphics::None
    }
}
