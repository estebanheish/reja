use ratatui::{
    layout::Margin,
    style::Style,
    widgets::{Block, Borders, Clear, Padding, Widget},
};
use tui_textarea::TextArea;

pub struct Prompt<'a>(pub TextArea<'a>);

impl<'a> Prompt<'a> {
    pub fn new() -> Self {
        let mut ta = TextArea::default();
        ta.set_cursor_line_style(Style::default());
        ta.set_cursor_style(Style::default().bg(ratatui::style::Color::Cyan));
        ta.set_block(Block::default().padding(Padding::new(3, 3, 1, 1)));
        Self(ta)
    }
}

impl<'a> Widget for Prompt<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        Clear.render(area, buf);
        self.0.widget().render(area, buf);
    }
}
