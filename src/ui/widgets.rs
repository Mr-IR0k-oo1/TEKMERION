//! UI widgets module

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Create a styled paragraph widget
pub fn styled_paragraph(text: &str, style: Style) -> Paragraph {
    Paragraph::new(text)
        .style(style)
        .wrap(Wrap { trim: true })
}

/// Create a bordered block widget
pub fn bordered_block(title: &str) -> Block {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
}
