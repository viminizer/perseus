use std::fmt;

use tui_textarea::{CursorMove, Input, Key, TextArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    Operator(char),
}

impl fmt::Display for VimMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
        }
    }
}

pub enum Transition {
    Nop,
    Mode(VimMode),
    Pending(Input),
    ExitField,
}

pub struct Vim {
    pub mode: VimMode,
    pending: Input,
}

impl Vim {
    pub fn new(mode: VimMode) -> Self {
        Self {
            mode,
            pending: Input::default(),
        }
    }

    fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
        }
    }

    pub fn transition(
        &self,
        input: Input,
        textarea: &mut TextArea<'_>,
        single_line: bool,
    ) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            VimMode::Normal | VimMode::Visual | VimMode::Operator(_) => {
                self.handle_normal_visual_operator(input, textarea, single_line)
            }
            VimMode::Insert => self.handle_insert(input, textarea, single_line),
        }
    }

    fn handle_insert(
        &self,
        input: Input,
        textarea: &mut TextArea<'_>,
        single_line: bool,
    ) -> Transition {
        match input {
            Input { key: Key::Esc, .. } => Transition::Mode(VimMode::Normal),
            Input {
                key: Key::Enter, ..
            } if single_line => Transition::Nop,
            input => {
                textarea.input(input);
                Transition::Mode(VimMode::Insert)
            }
        }
    }

    fn handle_normal_visual_operator(
        &self,
        input: Input,
        textarea: &mut TextArea<'_>,
        single_line: bool,
    ) -> Transition {
        match input {
            // Escape: exit field from Normal, cancel from Visual/Operator
            Input { key: Key::Esc, .. } => match self.mode {
                VimMode::Normal => Transition::ExitField,
                VimMode::Visual | VimMode::Operator(_) => {
                    textarea.cancel_selection();
                    Transition::Mode(VimMode::Normal)
                }
                _ => Transition::Nop,
            },
            // Basic motions
            Input {
                key: Key::Char('h'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Back);
                self.after_motion()
            }
            Input {
                key: Key::Char('j'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Down);
                self.after_motion()
            }
            Input {
                key: Key::Char('k'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Up);
                self.after_motion()
            }
            Input {
                key: Key::Char('l'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Forward);
                self.after_motion()
            }
            // Word motions
            Input {
                key: Key::Char('w'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordForward);
                self.after_motion()
            }
            Input {
                key: Key::Char('e'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordEnd);
                if matches!(self.mode, VimMode::Operator(_)) {
                    textarea.move_cursor(CursorMove::Forward);
                }
                self.after_motion()
            }
            Input {
                key: Key::Char('b'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordBack);
                self.after_motion()
            }
            // Line position motions
            Input {
                key: Key::Char('0'),
                ..
            }
            | Input {
                key: Key::Char('^'),
                ..
            } => {
                textarea.move_cursor(CursorMove::Head);
                self.after_motion()
            }
            Input {
                key: Key::Char('$'),
                ..
            } => {
                textarea.move_cursor(CursorMove::End);
                self.after_motion()
            }
            // gg: go to top (pending state for first g)
            Input {
                key: Key::Char('g'),
                ctrl: false,
                ..
            } if matches!(
                self.pending,
                Input {
                    key: Key::Char('g'),
                    ctrl: false,
                    ..
                }
            ) =>
            {
                textarea.move_cursor(CursorMove::Top);
                self.after_motion()
            }
            // G: go to bottom
            Input {
                key: Key::Char('G'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::Bottom);
                self.after_motion()
            }
            // Delete operations
            Input {
                key: Key::Char('x'),
                ctrl: false,
                ..
            } => {
                textarea.start_selection();
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('X'),
                ctrl: false,
                ..
            } => {
                textarea.start_selection();
                textarea.move_cursor(CursorMove::Back);
                textarea.cut();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('D'),
                ..
            } => {
                textarea.start_selection();
                let before = textarea.cursor();
                textarea.move_cursor(CursorMove::End);
                if before == textarea.cursor() {
                    textarea.move_cursor(CursorMove::Forward);
                }
                textarea.cut();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('C'),
                ..
            } => {
                textarea.start_selection();
                let before = textarea.cursor();
                textarea.move_cursor(CursorMove::End);
                if before == textarea.cursor() {
                    textarea.move_cursor(CursorMove::Forward);
                }
                textarea.cut();
                Transition::Mode(VimMode::Insert)
            }
            // Paste, undo, redo
            Input {
                key: Key::Char('p'),
                ctrl: false,
                ..
            } => {
                textarea.paste();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('u'),
                ctrl: false,
                ..
            } => {
                textarea.undo();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                textarea.redo();
                Transition::Mode(VimMode::Normal)
            }
            // Enter insert mode
            Input {
                key: Key::Char('i'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal => {
                textarea.cancel_selection();
                Transition::Mode(VimMode::Insert)
            }
            Input {
                key: Key::Char('a'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Forward);
                Transition::Mode(VimMode::Insert)
            }
            Input {
                key: Key::Char('A'),
                ..
            } if self.mode == VimMode::Normal => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::End);
                Transition::Mode(VimMode::Insert)
            }
            Input {
                key: Key::Char('I'),
                ..
            } if self.mode == VimMode::Normal => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Head);
                Transition::Mode(VimMode::Insert)
            }
            Input {
                key: Key::Char('o'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal && !single_line => {
                textarea.move_cursor(CursorMove::End);
                textarea.insert_newline();
                Transition::Mode(VimMode::Insert)
            }
            Input {
                key: Key::Char('O'),
                ..
            } if self.mode == VimMode::Normal && !single_line => {
                textarea.move_cursor(CursorMove::Head);
                textarea.insert_newline();
                textarea.move_cursor(CursorMove::Up);
                Transition::Mode(VimMode::Insert)
            }
            // Visual mode
            Input {
                key: Key::Char('v'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal => {
                textarea.start_selection();
                Transition::Mode(VimMode::Visual)
            }
            Input {
                key: Key::Char('V'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                textarea.move_cursor(CursorMove::End);
                Transition::Mode(VimMode::Visual)
            }
            // Cancel visual mode
            Input {
                key: Key::Char('v'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Visual => {
                textarea.cancel_selection();
                Transition::Mode(VimMode::Normal)
            }
            // Operator-pending: dd/yy/cc (same key doubles = operate on line)
            Input {
                key: Key::Char(c),
                ctrl: false,
                ..
            } if self.mode == VimMode::Operator(c) => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                let cursor = textarea.cursor();
                textarea.move_cursor(CursorMove::Down);
                if cursor == textarea.cursor() {
                    textarea.move_cursor(CursorMove::End);
                }
                self.complete_operator(c, textarea)
            }
            // Enter operator-pending mode
            Input {
                key: Key::Char(op @ ('y' | 'd' | 'c')),
                ctrl: false,
                ..
            } if self.mode == VimMode::Normal => {
                textarea.start_selection();
                Transition::Mode(VimMode::Operator(op))
            }
            // Visual mode operations
            Input {
                key: Key::Char('y'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.copy();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('d'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                Transition::Mode(VimMode::Normal)
            }
            Input {
                key: Key::Char('c'),
                ctrl: false,
                ..
            } if self.mode == VimMode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                Transition::Mode(VimMode::Insert)
            }
            // Scroll
            Input {
                key: Key::Char('d'),
                ctrl: true,
                ..
            } => {
                textarea.scroll((textarea.cursor().0.saturating_add(10) as i16, 0));
                Transition::Nop
            }
            Input {
                key: Key::Char('u'),
                ctrl: true,
                ..
            } => {
                textarea.scroll((-(textarea.cursor().0.min(10) as i16), 0));
                Transition::Nop
            }
            // Unhandled input becomes pending (for gg, etc.)
            input => Transition::Pending(input),
        }
    }

    fn after_motion(&self) -> Transition {
        match self.mode {
            VimMode::Operator(op) => self.complete_operator_noop(op),
            _ => Transition::Nop,
        }
    }

    fn complete_operator_noop(&self, op: char) -> Transition {
        // Motion completed the selection for the operator; delegate actual operation
        // We need to return that the operator should complete, but we can't mutate textarea here.
        // Instead, return the mode transition and let the caller handle it via complete_operator.
        Transition::Mode(VimMode::Operator(op))
    }

    fn complete_operator(&self, op: char, textarea: &mut TextArea<'_>) -> Transition {
        match op {
            'y' => {
                textarea.copy();
                Transition::Mode(VimMode::Normal)
            }
            'd' => {
                textarea.cut();
                Transition::Mode(VimMode::Normal)
            }
            'c' => {
                textarea.cut();
                Transition::Mode(VimMode::Insert)
            }
            _ => Transition::Mode(VimMode::Normal),
        }
    }

    pub fn apply_transition(self, transition: Transition, textarea: &mut TextArea<'_>) -> Self {
        match transition {
            Transition::Mode(new_mode) => {
                // If transitioning from Operator to same Operator (motion completed),
                // actually complete the operation
                if let VimMode::Operator(op) = self.mode {
                    if new_mode == self.mode {
                        match op {
                            'y' => {
                                textarea.copy();
                                return Vim::new(VimMode::Normal);
                            }
                            'd' => {
                                textarea.cut();
                                return Vim::new(VimMode::Normal);
                            }
                            'c' => {
                                textarea.cut();
                                return Vim::new(VimMode::Insert);
                            }
                            _ => return Vim::new(VimMode::Normal),
                        }
                    }
                }
                Vim::new(new_mode)
            }
            Transition::Pending(input) => self.with_pending(input),
            Transition::Nop | Transition::ExitField => Vim::new(self.mode),
        }
    }
}
