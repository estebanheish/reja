use ratatui::{crossterm::style::Color, widgets::Widget};

pub struct Markdown {
    pub content: String,
    pub scroll: i32,
}

impl Widget for Markdown {
    fn render(self, area: ratatui::prelude::Rect, _buf: &mut ratatui::prelude::Buffer) {
        let mut skin = termimad::MadSkin::default_dark();
        skin.bold.set_fg(Color::Blue);
        let mut mv = termimad::MadView::from(
            self.content,
            termimad::Area {
                left: area.left() + 3,
                top: area.top() + 1,
                width: area.width - 6,
                height: area.height - 2,
            },
            skin,
        );
        mv.try_scroll_lines(self.scroll);
        let _ = mv.write();
    }
}
