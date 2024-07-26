mod config;
mod markdown;
mod ollama;
use markdown::Markdown;
use ollama::{chat, Converation, Message};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Clear, Padding},
    Frame, Terminal,
};
use std::{
    io::{self, stdout, Result, Stdout},
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::RwLock;
use tui_textarea::TextArea;

type Tui = Terminal<CrosstermBackend<Stdout>>;

struct Reja<'a> {
    conversation: Arc<RwLock<ollama::Converation>>,
    last_rendered: String,
    exit: bool,
    rerender: bool,
    prompt: TextArea<'a>,
    cursor: usize,
    scroll: i32,
    send: bool,
    messages_len: usize,
    receiving: Arc<AtomicBool>,
    sysmsgs_len: usize,
}

fn default_prompt<'a>() -> TextArea<'a> {
    let mut ta = TextArea::default();
    ta.set_cursor_line_style(Style::default());
    ta.set_cursor_style(Style::default().bg(ratatui::style::Color::Cyan));
    ta.set_block(Block::default().padding(Padding::new(3, 3, 1, 0)));
    ta
}

impl Reja<'_> {
    fn new(conv: Converation) -> Self {
        Self {
            prompt: default_prompt(),
            last_rendered: "".to_string(),
            sysmsgs_len: conv.messages.len(),
            cursor: conv.messages.len(),
            conversation: Arc::new(RwLock::new(conv)),
            receiving: Arc::new(AtomicBool::new(false)),
            rerender: false,
            exit: false,
            send: false,
            scroll: 0,
            messages_len: 0,
        }
    }

    async fn run(&mut self, terminal: &mut Tui) -> io::Result<()> {
        while !self.exit {
            let message = self
                .conversation
                .read()
                .await
                .messages
                .get(self.cursor + 1)
                .cloned();
            terminal.draw(|f| self.render(f, message))?;
            self.handle_events()?;
            self.check_send().await;
        }
        Ok(())
    }

    async fn check_send(&mut self) {
        if self.send {
            if self.messages_len > 0 {
                self.cursor += 2;
            }
            chat(
                self.conversation.clone(),
                self.prompt.clone().into_lines().remove(0),
                self.receiving.clone(),
            )
            .await;
            self.messages_len += 2;
            self.prompt = default_prompt();
            self.send = false;
        }
    }

    fn render(&mut self, frame: &mut Frame, message: Option<Message>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(frame.size());
        if let Some(message) = message {
            if message.content != self.last_rendered || self.rerender {
                frame.render_widget(
                    Markdown {
                        content: message.content.clone(),
                        scroll: self.scroll,
                    },
                    frame.size(),
                );
                self.rerender = false;
                self.last_rendered.clone_from(&message.content);
            }
            if self.prompt.cursor() != (0, 0) {
                frame.render_widget(Clear, chunks[1]);
                frame.render_widget(self.prompt.widget(), chunks[1]);
            }
        } else {
            frame.render_widget(self.prompt.widget(), frame.size());
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                event::Event::Resize(_, _) => self.rerender = true,
                event::Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event);
                }
                event::Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => self.exit = true,
            KeyCode::Enter => self.send = true,
            KeyCode::Char(c) if c == 'q' && key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit = true
            }
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.receiving
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            }
            KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.prompt = default_prompt();
            }
            KeyCode::Up => {
                self.scroll -= 1;
                self.rerender = true;
            }
            KeyCode::Down => {
                self.scroll += 1;
                self.rerender = true;
            }
            KeyCode::Left => {
                if self.cursor > self.sysmsgs_len {
                    self.cursor -= 2;
                    self.rerender = true;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.messages_len - 2 {
                    self.cursor += 2;
                    self.rerender = true;
                }
            }
            _ => {
                self.prompt.input(key_event);
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            event::MouseEventKind::ScrollDown => {
                self.scroll += 1;
                self.rerender = true;
            }
            event::MouseEventKind::ScrollUp => {
                self.scroll -= 1;
                self.rerender = true;
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let p = config::profile();
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    Reja::new(Converation::from_profile(p))
        .run(&mut terminal)
        .await?;
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
