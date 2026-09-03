use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, EventLevel, PipelinePhase, PIPELINE_PHASES};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 40 || area.height < 15 {
        let msg = Paragraph::new("Terminal too small. Need 120x40 minimum.")
            .style(Style::default().fg(Color::Red));
        frame.render_widget(msg, area);
        return;
    }

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, main_layout[0]);
    render_progress(frame, main_layout[1], app);
    render_content(frame, main_layout[2], app);
    render_events(frame, main_layout[3], app);
    render_footer(frame, main_layout[4]);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title_line = Line::from(vec![
        Span::styled(
            "TEKMERION",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" // ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "EVIDENCE INTELLIGENCE ENGINE",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let header = Paragraph::new(title_line);
    frame.render_widget(header, area);
}

fn render_progress(frame: &mut Frame, area: Rect, app: &App) {
    let phase_label = app.phase.status_text();
    let progress_pct = (app.progress * 100.0) as u16;
    let label = format!(" {} [{}%] ", phase_label, progress_pct);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::BOTTOM))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .ratio(app.progress as f64)
        .label(Span::styled(label, Style::default().fg(Color::White)));
    frame.render_widget(gauge, area);
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Fill(1)])
        .split(area);

    render_pipeline(frame, content_layout[0], app);
    render_details(frame, content_layout[1], app);
}

fn render_pipeline(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = PIPELINE_PHASES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let current_idx = app.phase.index();
            let (marker, style) = if i < current_idx {
                ("[x] ", Style::default().fg(Color::Green))
            } else if i == current_idx && app.phase != PipelinePhase::Idle {
                (
                    "[>] ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("[ ] ", Style::default().fg(Color::DarkGray))
            };

            let line = Line::from(vec![
                Span::styled(marker, style),
                Span::styled(*name, style),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" PIPELINE ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_details(frame: &mut Frame, area: Rect, app: &App) {
    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Fill(1)])
        .split(area);

    render_candidate_list(frame, detail_layout[0], app);
    render_candidate_detail(frame, detail_layout[1], app);
}

fn render_candidate_list(frame: &mut Frame, area: Rect, app: &App) {
    if app.candidates.is_empty() {
        let empty = Paragraph::new("No candidates discovered yet")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(" CANDIDATES ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let sim_color = if c.similarity >= 0.9 {
                Color::Green
            } else if c.similarity >= 0.8 {
                Color::Yellow
            } else {
                Color::Red
            };
            let line = Line::from(vec![
                Span::styled(format!("{} ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(&c.title, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" [{:.0}%]", c.similarity * 100.0),
                    Style::default().fg(sim_color),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_candidate));

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" CANDIDATES [{}] ", app.candidates.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_candidate_detail(frame: &mut Frame, area: Rect, app: &App) {
    if app.candidates.is_empty() {
        render_pipeline_detail(frame, area, app);
    } else {
        render_candidate_info(frame, area, app);
    }
}

fn render_candidate_info(frame: &mut Frame, area: Rect, app: &App) {
    let Some(c) = app.candidates.get(app.selected_candidate) else {
        let empty = Paragraph::new("No candidate selected").block(
            Block::default()
                .title(" DETAIL ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(empty, area);
        return;
    };

    let sim_color = if c.similarity >= 0.9 {
        Color::Green
    } else if c.similarity >= 0.8 {
        Color::Yellow
    } else {
        Color::Red
    };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Title:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(&c.title, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Provider: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&c.provider, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  URL:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&c.url, Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("  Snippet:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&c.snippet, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Similarity: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}%", c.similarity * 100.0),
                Style::default().fg(sim_color),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Evidence & Blockchain Status:",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let mut detail_lines = lines;

    if !app.evidence_root.is_empty() {
        detail_lines.push(Line::from(vec![
            Span::styled("  Root: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.evidence_root, Style::default().fg(Color::Magenta)),
        ]));
    }
    if !app.tx_hash.is_empty() {
        detail_lines.push(Line::from(vec![
            Span::styled("  TX:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.tx_hash, Style::default().fg(Color::Magenta)),
        ]));
    }
    if !app.verification_result.is_empty() {
        let result_color = match app.verification_result.as_str() {
            "CONFIRMED" => Color::Green,
            "TAMPER DETECTED" => Color::Red,
            _ => Color::Yellow,
        };
        detail_lines.push(Line::from(vec![
            Span::styled("  Result: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &app.verification_result,
                Style::default()
                    .fg(result_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    let detail = Paragraph::new(detail_lines).wrap(Wrap { trim: true });
    let block = Block::default()
        .title(" DETAIL ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(detail.block(block), area);
}

fn render_pipeline_detail(frame: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Pipeline Overview",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Current Phase: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.phase.status_text(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Progress:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}%", app.progress * 100.0),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Candidates:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.candidates.len()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ENTER to start the pipeline",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let detail = Paragraph::new(lines);
    let block = Block::default()
        .title(" DETAIL ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(detail.block(block), area);
}

fn render_events(frame: &mut Frame, area: Rect, app: &App) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.events.len();
    let start = total.saturating_sub(visible_height);

    let events: Vec<Line> = app.events[start..]
        .iter()
        .map(|e| {
            let level_color = match e.level {
                EventLevel::Info => Color::DarkGray,
                EventLevel::Success => Color::Green,
                EventLevel::Warning => Color::Yellow,
                EventLevel::Error => Color::Red,
            };
            let ts = e.timestamp.format("%H:%M:%S");
            Line::from(vec![
                Span::styled(format!("{} ", ts), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("[{:?}] ", e.level),
                    Style::default().fg(level_color),
                ),
                Span::styled(&e.message, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let events = if events.is_empty() {
        vec![Line::from(Span::styled(
            "  No events yet",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        events
    };

    let paragraph = Paragraph::new(events).wrap(Wrap { trim: true });
    let block = Block::default()
        .title(" EVENTS ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(paragraph.block(block), area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let keys = Line::from(vec![
        Span::styled(
            " ENTER",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Start  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "V",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Verify  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "T",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Tamper  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "R",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Reset  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "\u{2191}\u{2193}",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Select  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
    ]);
    let footer = Paragraph::new(keys);
    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn setup_terminal(width: u16, height: u16) -> Terminal<TestBackend> {
        let backend = TestBackend::new(width, height);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_idle() {
        let mut terminal = setup_terminal(120, 40);
        let app = App::new();
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_with_candidates() {
        let mut terminal = setup_terminal(120, 40);
        let mut app = App::new();
        app.phase = crate::app::PipelinePhase::Verify;
        app.candidates = vec![crate::app::Candidate {
            title: "Test".into(),
            provider: "Google".into(),
            url: "https://test.com".into(),
            snippet: "A test snippet".into(),
            similarity: 0.95,
        }];
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_evidence_phase() {
        let mut terminal = setup_terminal(120, 40);
        let mut app = App::new();
        app.phase = crate::app::PipelinePhase::FinalVerify;
        app.evidence_root = "0x7a3b1234567890".into();
        app.tx_hash = "0xabcdef12345678".into();
        app.verification_result = "CONFIRMED".into();
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_tamper() {
        let mut terminal = setup_terminal(120, 40);
        let mut app = App::new();
        app.phase = crate::app::PipelinePhase::FinalVerify;
        app.evidence_root = "TAMPERED_0x7a3b".into();
        app.verification_result = "TAMPER DETECTED".into();
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_too_small() {
        let mut terminal = setup_terminal(30, 10);
        let app = App::new();
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_empty_candidates() {
        let mut terminal = setup_terminal(120, 40);
        let mut app = App::new();
        app.phase = crate::app::PipelinePhase::Discovery;
        terminal.draw(|f| render(f, &app)).unwrap();
    }

    #[test]
    fn test_render_many_events() {
        let mut terminal = setup_terminal(120, 40);
        let mut app = App::new();
        for i in 0..50 {
            app.push_event(format!("Event number {i}"), crate::app::EventLevel::Info);
        }
        terminal.draw(|f| render(f, &app)).unwrap();
    }
}
