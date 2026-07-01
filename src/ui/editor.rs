use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use tui_textarea::{CursorMove, Input, Key, TextArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Command,
}

impl VimMode {
    pub fn label(self) -> &'static str {
        match self {
            VimMode::Normal => "NORMAL",
            VimMode::Insert => "INSERT",
            VimMode::Command => "COMMAND",
        }
    }
}

pub enum EditorSignal {
    None,
    Save,
    Quit,
    SaveAndQuit,
}

pub struct Editor {
    pub textarea: TextArea<'static>,
    pub mode: VimMode,
    pub skill_md: PathBuf,
    pub skill_name: String,
    pub command: String,
    pub dirty: bool,
    pub pending_g: bool,
    pub pending_op: Option<char>,
    pub yank: String,
}

impl Editor {
    pub fn new(skill_md: PathBuf, skill_name: String, body: &str) -> Editor {
        let lines: Vec<String> = if body.is_empty() {
            vec![String::new()]
        } else {
            body.lines().map(|l| l.to_string()).collect()
        };
        let mut textarea = TextArea::new(lines);
        textarea.set_cursor_line_style(Default::default());
        Editor {
            textarea,
            mode: VimMode::Normal,
            skill_md,
            skill_name,
            command: String::new(),
            dirty: false,
            pending_g: false,
            pending_op: None,
            yank: String::new(),
        }
    }

    pub fn body(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> EditorSignal {
        match self.mode {
            VimMode::Insert => self.handle_insert(key),
            VimMode::Normal => self.handle_normal(key),
            VimMode::Command => self.handle_command(key),
        }
    }

    fn handle_insert(&mut self, key: KeyEvent) -> EditorSignal {
        if key.code == KeyCode::Esc {
            self.mode = VimMode::Normal;
            return EditorSignal::None;
        }
        let input = to_input(key);
        if self.textarea.input(input) {
            self.dirty = true;
        }
        EditorSignal::None
    }

    fn handle_command(&mut self, key: KeyEvent) -> EditorSignal {
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                self.command.clear();
            }
            KeyCode::Enter => {
                let cmd = self.command.trim().to_string();
                self.command.clear();
                self.mode = VimMode::Normal;
                return self.run_command(&cmd);
            }
            KeyCode::Backspace => {
                self.command.pop();
                if self.command.is_empty() {
                    self.mode = VimMode::Normal;
                }
            }
            KeyCode::Char(c) => self.command.push(c),
            _ => {}
        }
        EditorSignal::None
    }

    fn run_command(&mut self, cmd: &str) -> EditorSignal {
        match cmd {
            "w" => EditorSignal::Save,
            "q" => EditorSignal::Quit,
            "wq" | "x" => EditorSignal::SaveAndQuit,
            "q!" => EditorSignal::Quit,
            _ => EditorSignal::None,
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> EditorSignal {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if self.pending_g {
            self.pending_g = false;
            if let KeyCode::Char('g') = key.code {
                self.textarea.move_cursor(CursorMove::Top);
                return EditorSignal::None;
            }
        }

        if let Some(op) = self.pending_op {
            self.pending_op = None;
            return self.handle_operator(op, key);
        }

        match key.code {
            KeyCode::Char('i') => self.mode = VimMode::Insert,
            KeyCode::Char('I') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('a') => {
                self.textarea.move_cursor(CursorMove::Forward);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('A') => {
                self.textarea.move_cursor(CursorMove::End);
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('o') => {
                self.textarea.move_cursor(CursorMove::End);
                self.textarea.insert_newline();
                self.mode = VimMode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('O') => {
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.insert_newline();
                self.textarea.move_cursor(CursorMove::Up);
                self.mode = VimMode::Insert;
                self.dirty = true;
            }
            KeyCode::Char('d') if ctrl => {
                for _ in 0..10 {
                    self.textarea.move_cursor(CursorMove::Down);
                }
            }
            KeyCode::Char('u') if ctrl => {
                for _ in 0..10 {
                    self.textarea.move_cursor(CursorMove::Up);
                }
            }
            KeyCode::Char('r') if ctrl => {
                self.textarea.redo();
            }
            KeyCode::Char('h') | KeyCode::Left => self.textarea.move_cursor(CursorMove::Back),
            KeyCode::Char('l') | KeyCode::Right => self.textarea.move_cursor(CursorMove::Forward),
            KeyCode::Char('j') | KeyCode::Down => self.textarea.move_cursor(CursorMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.textarea.move_cursor(CursorMove::Up),
            KeyCode::Char('w') => self.textarea.move_cursor(CursorMove::WordForward),
            KeyCode::Char('b') => self.textarea.move_cursor(CursorMove::WordBack),
            KeyCode::Char('0') | KeyCode::Home => self.textarea.move_cursor(CursorMove::Head),
            KeyCode::Char('$') | KeyCode::End => self.textarea.move_cursor(CursorMove::End),
            KeyCode::Char('g') => self.pending_g = true,
            KeyCode::Char('G') => self.textarea.move_cursor(CursorMove::Bottom),
            KeyCode::Char('d') => self.pending_op = Some('d'),
            KeyCode::Char('y') => self.pending_op = Some('y'),
            KeyCode::Char('D') => {
                self.textarea.delete_line_by_end();
                self.dirty = true;
            }
            KeyCode::Char('x') => {
                self.textarea.delete_next_char();
                self.dirty = true;
            }
            KeyCode::Char('p') => {
                if !self.yank.is_empty() {
                    self.textarea.move_cursor(CursorMove::End);
                    self.textarea.insert_newline();
                    self.textarea.insert_str(self.yank.trim_end_matches('\n'));
                    self.dirty = true;
                }
            }
            KeyCode::Char('u') => {
                self.textarea.undo();
            }
            KeyCode::Char(':') => {
                self.mode = VimMode::Command;
                self.command.clear();
            }
            KeyCode::Esc => {}
            _ => {}
        }
        EditorSignal::None
    }

    fn handle_operator(&mut self, op: char, key: KeyEvent) -> EditorSignal {
        match (op, key.code) {
            ('d', KeyCode::Char('d')) => {
                let line = self.current_line();
                self.yank = format!("{line}\n");
                self.textarea.move_cursor(CursorMove::Head);
                self.textarea.delete_line_by_end();
                self.textarea.delete_newline();
                self.dirty = true;
            }
            ('y', KeyCode::Char('y')) => {
                self.yank = format!("{}\n", self.current_line());
            }
            ('d', KeyCode::Char('w')) => {
                self.textarea.delete_next_word();
                self.dirty = true;
            }
            _ => {}
        }
        EditorSignal::None
    }

    fn current_line(&self) -> String {
        let (row, _) = self.textarea.cursor();
        self.textarea.lines().get(row).cloned().unwrap_or_default()
    }
}

fn to_input(key: KeyEvent) -> Input {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let k = match key.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Tab => Key::Tab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Esc => Key::Esc,
        _ => Key::Null,
    };
    Input {
        key: k,
        ctrl,
        alt,
        shift: key.modifiers.contains(KeyModifiers::SHIFT),
    }
}
