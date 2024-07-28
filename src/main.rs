mod config;
mod markdown;
mod ollama;
mod prompt;
use markdown::Markdown;
use ollama::{chat, Converation, Message};
use prompt::Prompt;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    Frame, Terminal,
};
use std::{
    io::{self, stdout, Result, Stdout},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
        Arc,
    },
};
use tokio::sync::RwLock;

type Tui = Terminal<CrosstermBackend<Stdout>>;

struct Reja<'a> {
    conversation: Arc<RwLock<ollama::Converation>>,
    last_rendered: String,
    exit: bool,
    rerender: bool,
    prompt: Prompt<'a>,
    cursor: usize,
    scroll: Arc<AtomicUsize>,
    send: bool,
    messages_len: usize,
    receiving: Arc<AtomicBool>,
    sysmsgs_len: usize,
    markdown: Markdown,
}

impl Reja<'_> {
    fn new(conv: Converation) -> Self {
        let scroll_atomic = Arc::new(AtomicUsize::new(0));
        let height = Arc::new(AtomicUsize::new(0));
        Self {
            prompt: Prompt::new(),
            last_rendered: "".to_string(),
            sysmsgs_len: conv.messages.len(),
            cursor: conv.messages.len(),
            conversation: Arc::new(RwLock::new(conv)),
            receiving: Arc::new(AtomicBool::new(false)),
            rerender: false,
            exit: false,
            send: false,
            scroll: scroll_atomic.clone(),
            messages_len: 0,
            markdown: Markdown::new("".to_string(), scroll_atomic, height),
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
                self.prompt.0.clone().into_lines().remove(0),
                self.receiving.clone(),
            )
            .await;
            self.messages_len += 2;
            self.prompt = Prompt::new();
            self.send = false;
        }
    }

    fn render(&mut self, frame: &mut Frame, message: Option<Message>) {
        let area = frame.size();
        let h = self.markdown.height.load(SeqCst) as u16 + 1;
        let k = if h > area.height - 3 { h - 3 } else { h };
        let layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Max(k),
                ratatui::layout::Constraint::Min(3),
            ])
            .split(frame.size());
        if let Some(message) = message {
            if message.content != self.last_rendered || self.rerender {
                self.markdown.content.clone_from(&message.content);
                frame.render_widget(&self.markdown, frame.size());
                self.rerender = false;
                self.last_rendered.clone_from(&message.content);
            }
            if !self.prompt.0.is_empty() {
                frame.render_widget(self.prompt.0.widget(), layout[1]);
            }
        } else {
            frame.render_widget(self.prompt.0.widget(), frame.size());
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
        let receiving = self.receiving.load(SeqCst);
        match key_event.code {
            KeyCode::Esc => self.exit = true,
            KeyCode::Enter if !receiving => self.send = true,
            KeyCode::Char(c) if c == 'q' && key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit = true
            }
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.receiving.store(false, SeqCst);
            }
            KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.prompt.0.is_empty() {
                    self.scroll_up(5);
                } else {
                    self.prompt = Prompt::new();
                }
            }
            KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_down(5)
            }
            KeyCode::Up => self.scroll_up(1),
            KeyCode::Down => self.scroll_down(1),
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
                if !receiving {
                    self.prompt.0.input(key_event);
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        match mouse_event.kind {
            event::MouseEventKind::ScrollDown => self.scroll_down(1),
            event::MouseEventKind::ScrollUp => self.scroll_up(1),
            _ => {}
        }
    }

    fn scroll_up(&mut self, offset: usize) {
        let k = self.scroll.load(SeqCst);
        if k > offset {
            self.scroll.store(k - offset, SeqCst);
        } else {
            self.scroll.store(0, SeqCst);
        }
        self.rerender = true;
    }

    fn scroll_down(&mut self, offset: usize) {
        let k = self.scroll.load(SeqCst);
        self.scroll.store(k + offset, SeqCst);
        self.rerender = true;
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
