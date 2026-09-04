use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use tekmerion_face::{BlurLevel, FaceQualityAssessment, QualityStatus};

use crate::app::{App, AppStatus, Stage};

/// Render the whole interface into a single frame.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Guard against very small terminal windows
    if area.width < 50 || area.height < 14 {
        render_too_small(frame, area);
        return;
    }

    // Dynamic height allocation for events so middle pane has at least 11 rows
    // (enough to comfortably display all 7 stages and all detail fields without clipping)
    let events_height = if area.height >= 30 {
        8
    } else if area.height >= 24 {
        6
    } else {
        5
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),             // header
        Constraint::Min(11),               // stage + detail
        Constraint::Length(events_height), // events
        Constraint::Length(3),             // footer
    ])
    .split(area);

    render_header(frame, chunks[0]);

    // Give the stages panel fixed width (28 cols is optimal for label + marker + padding)
    // and let the detail pane expand into the remaining width.
    let middle = Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)])
        .split(chunks[1]);
    render_stages(frame, middle[0], app);
    render_detail(frame, middle[1], app);

    render_events(frame, chunks[2], app);
    render_footer(frame, chunks[3]);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let warning = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Terminal window is too small for Tekmerion TUI",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Current: {}x{}  |  Minimum: 50x14", area.width, area.height),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Please enlarge your terminal window.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(warning, area);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " TEKMERION ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "// EVIDENCE INTELLIGENCE ENGINE  ",
            Style::default().fg(Color::White),
        ),
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .alignment(Alignment::Left);
    frame.render_widget(title, area);
}

fn render_stages(frame: &mut Frame, area: Rect, app: &App) {
    let lines: Vec<Line> = Stage::ALL
        .iter()
        .enumerate()
        .map(|(idx, stage)| stage_line(idx, *stage, app))
        .collect();

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("PIPELINE", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn stage_line(idx: usize, stage: Stage, app: &App) -> Line<'_> {
    let (marker, label_style) = stage_marker(idx, stage, app);
    let label = stage.label();

    if app.status == AppStatus::Running && app.current == Some(stage) {
        Line::from(vec![
            Span::raw(" "),
            marker,
            Span::raw(" "),
            Span::styled(format!("{:<13}", label), label_style),
            Span::styled(" [ACTIVE]", Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM)),
        ])
    } else {
        Line::from(vec![
            Span::raw(" "),
            marker,
            Span::raw(" "),
            Span::styled(format!("{:<13}", label), label_style),
        ])
    }
}

fn stage_marker(idx: usize, stage: Stage, app: &App) -> (Span<'static>, Style) {
    let done_upto = match (app.status, app.current) {
        (AppStatus::Completed, _) => Some(Stage::ALL.len()),
        (AppStatus::Running, Some(current)) => Some(current.index()),
        (AppStatus::Tampered, _) => Some(0),
        _ => None,
    };

    if app.status == AppStatus::Running && app.current == Some(stage) {
        (
            Span::styled(
                "▶",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )
    } else if app.status == AppStatus::Tampered && app.current == Some(stage) {
        (
            Span::styled(
                "✖",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else if done_upto.is_some_and(|n| idx < n) {
        (
            Span::styled("✓", Style::default().fg(Color::Green)),
            Style::default().fg(Color::Green),
        )
    } else {
        (
            Span::styled("○", Style::default().fg(Color::DarkGray)),
            Style::default().fg(Color::DarkGray),
        )
    }
}

fn render_face_quality(frame: &mut Frame, area: Rect, app: &App) {
    let quality = app
        .face_quality
        .as_ref()
        .cloned()
        .unwrap_or_else(FaceQualityAssessment::sample_good);

    let status_style = match quality.status {
        QualityStatus::Good => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        QualityStatus::Warning => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        QualityStatus::Reject => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let blur_style = match quality.blur.level {
        BlurLevel::Low => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        BlurLevel::Medium => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        BlurLevel::High => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let quality_style = if quality.overall_quality >= 0.75 {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else if quality.overall_quality >= 0.50 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("FACE QUALITY", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);

    if inner.height >= 19 {
        // Taller viewports: single vertical layout with blank line separators
        let lines = vec![
            Line::from(Span::styled(
                "FACE QUALITY",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("Faces:", key_style)),
            Line::from(Span::styled(quality.faces_display(), val_style)),
            Line::from(""),
            Line::from(Span::styled("Resolution:", key_style)),
            Line::from(Span::styled(quality.resolution_display(), val_style)),
            Line::from(""),
            Line::from(Span::styled("Blur:", key_style)),
            Line::from(Span::styled(quality.blur_display(), blur_style)),
            Line::from(""),
            Line::from(Span::styled("Pose:", key_style)),
            Line::from(Span::styled(quality.pose_display(), val_style)),
            Line::from(""),
            Line::from(Span::styled("Quality:", key_style)),
            Line::from(Span::styled(quality.quality_display(), quality_style)),
            Line::from(""),
            Line::from(Span::styled("Status:", key_style)),
            Line::from(Span::styled(quality.status_display(), status_style)),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    } else {
        // Standard viewports (e.g. 80x24): two-column layout to prevent clipping
        let rows = Layout::vertical([
            Constraint::Length(1), // Header
            Constraint::Fill(1),   // Fields
        ])
        .split(inner);

        let header = Paragraph::new(Line::from(Span::styled(
            "FACE QUALITY",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        frame.render_widget(header, rows[0]);

        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        let spaced = rows[1].height >= 8;

        let mut left_lines = Vec::new();
        left_lines.push(Line::from(Span::styled("Faces:", key_style)));
        left_lines.push(Line::from(Span::styled(quality.faces_display(), val_style)));
        if spaced {
            left_lines.push(Line::from(""));
        }
        left_lines.push(Line::from(Span::styled("Resolution:", key_style)));
        left_lines.push(Line::from(Span::styled(quality.resolution_display(), val_style)));
        if spaced {
            left_lines.push(Line::from(""));
        }
        left_lines.push(Line::from(Span::styled("Blur:", key_style)));
        left_lines.push(Line::from(Span::styled(quality.blur_display(), blur_style)));

        let mut right_lines = Vec::new();
        right_lines.push(Line::from(Span::styled("Pose:", key_style)));
        right_lines.push(Line::from(Span::styled(quality.pose_display(), val_style)));
        if spaced {
            right_lines.push(Line::from(""));
        }
        right_lines.push(Line::from(Span::styled("Quality:", key_style)));
        right_lines.push(Line::from(Span::styled(quality.quality_display(), quality_style)));
        if spaced {
            right_lines.push(Line::from(""));
        }
        right_lines.push(Line::from(Span::styled("Status:", key_style)));
        right_lines.push(Line::from(Span::styled(quality.status_display(), status_style)));

        frame.render_widget(Paragraph::new(left_lines), cols[0]);
        frame.render_widget(Paragraph::new(right_lines), cols[1]);
    }

    frame.render_widget(block, area);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    if app.current == Some(Stage::Face) {
        render_face_quality(frame, area, app);
        return;
    }

    let (status_badge, status_style) = match app.status {
        AppStatus::Idle => (
            " IDLE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Running => (
            " RUNNING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Completed => (
            " COMPLETED ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Tampered => (
            " TAMPERED ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let result_style = match app.verification_result.as_str() {
        "verified" => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        "tampered" => Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD),
        _ => Style::default().fg(Color::DarkGray),
    };

    let selected = if app.candidate_count == 0 {
        "none".to_string()
    } else {
        format!("{}/{}", app.selected_candidate + 1, app.candidate_count)
    };

    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:<15}", "STATUS:"), key_style),
            Span::styled(status_badge, status_style),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "STATE:"), key_style),
            Span::styled(app.pipeline_state().label(), val_style),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "CANDIDATES:"), key_style),
            Span::styled(format!("{}", app.candidate_count), val_style),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "SELECTED:"), key_style),
            Span::styled(selected, val_style),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "EVIDENCE ROOT:"), key_style),
            if app.evidence_root == "--" {
                Span::styled("--", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(&app.evidence_root, Style::default().fg(Color::Cyan))
            },
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "TX HASH:"), key_style),
            if app.tx_hash == "--" {
                Span::styled("--", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled(&app.tx_hash, Style::default().fg(Color::Cyan))
            },
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "RESULT:"), key_style),
            Span::styled(&app.verification_result, result_style),
        ]),
    ];

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("DETAILS", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(inner);

    frame.render_widget(Paragraph::new(lines), rows[0]);

    let gauge_color = match app.status {
        AppStatus::Idle => Color::DarkGray,
        AppStatus::Running => Color::Cyan,
        AppStatus::Completed => Color::Green,
        AppStatus::Tampered => Color::Red,
    };

    let percent = app.progress_percent();
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(gauge_color)
                .bg(Color::Rgb(25, 25, 30))
                .add_modifier(Modifier::BOLD),
        )
        .percent(percent)
        .label(Span::styled(
            format!("Progress: {}%", percent),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(gauge, rows[1]);
    frame.render_widget(block, area);
}

fn render_events(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("RECENT EVENTS", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.events.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            "  (No events recorded yet. Press [ENTER] to start pipeline)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
        frame.render_widget(placeholder, inner);
        return;
    }

    // Auto-scroll to show the most recent events that fit in the pane
    let visible_lines = inner.height as usize;
    let skip = app.events.len().saturating_sub(visible_lines);

    let lines: Vec<Line> = app
        .events
        .iter()
        .skip(skip)
        .map(|event| {
            let (icon, icon_style) = if event.contains("verified") {
                ("★ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else if event.contains("complete") {
                ("✓ ", Style::default().fg(Color::Green))
            } else if event.contains("started") {
                ("▶ ", Style::default().fg(Color::Cyan))
            } else if event.contains("Tamper") {
                ("✖ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            } else if event.contains("reset") {
                ("↺ ", Style::default().fg(Color::Yellow))
            } else {
                ("• ", Style::default().fg(Color::White))
            };

            Line::from(vec![
                Span::raw(" "),
                Span::styled(icon, icon_style),
                Span::styled(event.as_str(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            " ENTER ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Start  ", Style::default().fg(Color::White)),
        Span::styled(
            " V ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Advance  ", Style::default().fg(Color::White)),
        Span::styled(
            " T ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Tamper  ", Style::default().fg(Color::White)),
        Span::styled(
            " R ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Reset  ", Style::default().fg(Color::White)),
        Span::styled(
            " ↑/↓ ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Select  ", Style::default().fg(Color::White)),
        Span::styled(
            " Q ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Quit", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
