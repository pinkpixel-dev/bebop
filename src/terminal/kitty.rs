use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::io::{self, Write};

pub struct KittyRenderer;

impl KittyRenderer {
    /// Render a PNG image buffer using Kitty graphics protocol at terminal cell position (col, row).
    pub fn render_png(png_bytes: &[u8], col: u16, row: u16, width_cells: u16, height_cells: u16) -> io::Result<()> {
        let b64 = BASE64.encode(png_bytes);
        let mut stdout = io::stdout();

        // Move cursor to top-left of artwork area
        write!(stdout, "\x1b[{};{}H", row + 1, col + 1)?;

        let chunk_size = 4096;
        let total_len = b64.len();
        let mut offset = 0;
        let mut is_first = true;

        while offset < total_len {
            let end = (offset + chunk_size).min(total_len);
            let chunk = &b64[offset..end];
            let is_last = end >= total_len;
            let m_flag = if is_last { 0 } else { 1 };

            if is_first {
                // a=T (transmit and display), f=100 (PNG format), c=columns, r=rows, q=2 (suppress response)
                write!(
                    stdout,
                    "\x1b_Ga=T,f=100,c={},r={},q=2,m={};{}\x1b\\",
                    width_cells, height_cells, m_flag, chunk
                )?;
                is_first = false;
            } else {
                write!(stdout, "\x1b_Gm={};{}\x1b\\", m_flag, chunk)?;
            }

            offset = end;
        }

        stdout.flush()?;
        Ok(())
    }

    /// Clear all displayed Kitty images.
    pub fn clear_all() -> io::Result<()> {
        let mut stdout = io::stdout();
        write!(stdout, "\x1b_Ga=d,d=a,q=2\x1b\\")?;
        stdout.flush()?;
        Ok(())
    }
}
