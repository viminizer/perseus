use std::fmt;

use arboard::Clipboard;

#[derive(Debug)]
pub enum ClipboardError {
    Init(arboard::Error),
    Read(arboard::Error),
    Write(arboard::Error),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::Init(err) => write!(f, "init failed: {err}"),
            ClipboardError::Read(err) => write!(f, "read failed: {err}"),
            ClipboardError::Write(err) => write!(f, "write failed: {err}"),
        }
    }
}

pub struct ClipboardProvider {
    clipboard: Option<Clipboard>,
}

impl ClipboardProvider {
    pub fn new() -> Self {
        Self {
            clipboard: Clipboard::new().ok(),
        }
    }

    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new().map_err(ClipboardError::Init)?);
        }
        self.clipboard
            .as_mut()
            .expect("clipboard must be initialized")
            .get_text()
            .map_err(ClipboardError::Read)
    }

    pub fn set_text(&mut self, text: String) -> Result<(), ClipboardError> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new().map_err(ClipboardError::Init)?);
        }
        self.clipboard
            .as_mut()
            .expect("clipboard must be initialized")
            .set_text(text)
            .map_err(ClipboardError::Write)
    }
}
