use std::sync::{atomic::AtomicUsize, Arc};

use ratatui::prelude::*;
use ratatui::widgets::Widget;
use termimad::{MadSkin, TextView};

#[derive(Clone)]
pub struct Markdown {
    pub content: String,
    pub skin: MadSkin,
    pub scroll: Arc<AtomicUsize>,
    pub height: Arc<AtomicUsize>,
}

impl Markdown {
    pub fn new(content: String, scroll: Arc<AtomicUsize>, height: Arc<AtomicUsize>) -> Self {
        Self {
            content,
            scroll,
            height,
            skin: MadSkin::default_dark(),
        }
    }

    fn write(&self, area: Rect) {
        let area = termimad::Area {
            left: area.left() + 3,
            top: area.top() + 1,
            width: area.width - 6,
            height: area.height - 2,
        };
        let text = self.skin.area_text(&self.content, &area);
        let mut view = TextView::from(&area, &text);
        let scroll = self.scroll.load(std::sync::atomic::Ordering::SeqCst);
        let new_scroll = view.set_scroll(scroll);
        self.scroll
            .store(new_scroll, std::sync::atomic::Ordering::SeqCst);
        self.height
            .store(view.content_height(), std::sync::atomic::Ordering::SeqCst);
        let _ = view.write();
    }
}

impl Widget for &Markdown {
    fn render(self, area: ratatui::prelude::Rect, _buf: &mut ratatui::prelude::Buffer) {
        self.write(area);
    }
}
