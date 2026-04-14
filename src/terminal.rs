use std::io::Stdout;

use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};

pub struct TerminalSession {
    keyboard_enhancements: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalGeometry {
    pub cols: u16,
    pub rows: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl TerminalSession {
    pub fn enter(stdout: &mut Stdout) -> Result<Self> {
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, Hide)?;

        let keyboard_enhancements = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )
        .is_ok();

        Ok(Self {
            keyboard_enhancements,
        })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        if self.keyboard_enhancements {
            let _ = execute!(stdout, PopKeyboardEnhancementFlags);
        }
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn geometry() -> Result<TerminalGeometry> {
    let (cols, rows) = size()?;
    let (pixel_width, pixel_height) = pixel_size();

    Ok(TerminalGeometry {
        cols,
        rows,
        pixel_width,
        pixel_height,
    })
}

#[cfg(unix)]
fn pixel_size() -> (u16, u16) {
    use std::os::fd::AsRawFd;

    let stdout = std::io::stdout();
    let fd = stdout.as_raw_fd();
    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize) };
    if result == 0 {
        (winsize.ws_xpixel, winsize.ws_ypixel)
    } else {
        (0, 0)
    }
}

#[cfg(not(unix))]
fn pixel_size() -> (u16, u16) {
    (0, 0)
}
