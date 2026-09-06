use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph},
    Frame,
};

use tekmerion_core::VerificationStatus;
use tekmerion_face::{BlurLevel, FaceQualityAssessment, QualityStatus};
use tekmerion_verification::RankedCandidate;

use crate::app::{App, AppStatus, Stage, ViewTab};

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
        Constraint::Min(11),               // stage + detail / active tab
        Constraint::Length(events_height), // events
        Constraint::Length(3),             // footer
    ])
    .split(area);

    render_header(frame, chunks[0], app);

    // Render active tab view
    match app.active_tab {
        ViewTab::Pipeline => {
            // Give the stages panel fixed width (28 cols is optimal for label + marker + padding)
            // and let the detail pane expand into the remaining width.
            let middle = Layout::horizontal([Constraint::Length(28), Constraint::Fill(1)]).split(chunks[1]);
            render_stages(frame, middle[0], app);
            render_detail(frame, middle[1], app);
        }
        ViewTab::Evidence => {
            render_tab_evidence(frame, chunks[1], app);
        }
        ViewTab::Candidates => {
            render_tab_candidates(frame, chunks[1], app);
        }
        ViewTab::Guide => {
            render_tab_guide(frame, chunks[1], app);
        }
    }

    render_events(frame, chunks[2], app);
    render_footer(frame, chunks[3]);

    // Overlay help modal if active
    if app.show_help {
        render_help_modal(frame, area);
    }
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let warning = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "Terminal window is too small for Tekmerion TUI",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
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

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let status_pill = match app.status {
        AppStatus::Idle => Span::styled(
            " ● STANDBY ",
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Rgb(20, 35, 45))
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Running => Span::styled(
            " ⚡ LIVE FLOW ",
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(45, 40, 15))
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Completed => Span::styled(
            " ✓ VERIFIED ",
            Style::default()
                .fg(Color::Green)
                .bg(Color::Rgb(15, 40, 20))
                .add_modifier(Modifier::BOLD),
        ),
        AppStatus::Tampered => Span::styled(
            " ✖ TAMPER ALERT ",
            Style::default()
                .fg(Color::Red)
                .bg(Color::Rgb(45, 15, 15))
                .add_modifier(Modifier::BOLD),
        ),
    };

    let mut title_spans = vec![
        Span::styled(
            " ◈ TEKMERION ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "// EVIDENCE INTELLIGENCE ENGINE  ",
            Style::default().fg(Color::White),
        ),
        status_pill,
        Span::raw(" "),
    ];

    if area.width >= 75 {
        title_spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));

        for tab in ViewTab::ALL {
            let is_active = app.active_tab == tab;
            let tab_label = match tab {
                ViewTab::Pipeline => "1:Flow",
                ViewTab::Evidence => "2:Evidence",
                ViewTab::Candidates => "3:Candidates",
                ViewTab::Guide => "4:Guide",
            };

            if is_active {
                title_spans.push(Span::styled(
                    format!(" ▰ {} ", tab_label),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                title_spans.push(Span::styled(
                    format!(" {} ", tab_label),
                    Style::default().fg(Color::Gray),
                ));
            }
            title_spans.push(Span::raw(" "));
        }

        title_spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
        title_spans.push(Span::styled(
            "[?:Help]",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
    }

    let title = Paragraph::new(Line::from(title_spans))
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
    let mut lines = Vec::new();

    let step_badge = match app.status {
        AppStatus::Running => {
            let curr_idx = app.current.map(|s| s.index() + 1).unwrap_or(0);
            format!("STEP {}/{}", curr_idx, Stage::ALL.len())
        }
        AppStatus::Completed => "7/7 COMPLETE".to_string(),
        AppStatus::Tampered => "ALERT".to_string(),
        AppStatus::Idle => "READY".to_string(),
    };

    let title_block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("PIPELINE", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!(" [{}]", step_badge), Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = title_block.inner(area);

    for (idx, stage) in Stage::ALL.iter().enumerate() {
        lines.push(stage_stepper_line(idx, *stage, app));

        if inner.height >= 14 && idx + 1 < Stage::ALL.len() {
            let rail_color = if let Some(curr) = app.current {
                if idx < curr.index() {
                    Color::Green
                } else if idx == curr.index() {
                    Color::Yellow
                } else {
                    Color::DarkGray
                }
            } else if app.status == AppStatus::Completed {
                Color::Green
            } else {
                Color::DarkGray
            };
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("│", Style::default().fg(rail_color)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(title_block), area);
}

fn stage_stepper_line(idx: usize, stage: Stage, app: &App) -> Line<'_> {
    let (marker, label_style) = stage_marker(idx, stage, app);
    let label = stage.label();
    let num = idx + 1;

    if app.status == AppStatus::Running && app.current == Some(stage) {
        Line::from(vec![
            Span::raw(" "),
            marker,
            Span::styled(format!(" {:02} ", num), Style::default().fg(Color::Yellow)),
            Span::styled(format!("{:<11}", label), label_style),
            Span::styled(
                "[ACTIVE]",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::raw(" "),
            marker,
            Span::styled(format!(" {:02} ", num), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:<11}", label), label_style),
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
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    } else if app.status == AppStatus::Tampered && app.current == Some(stage) {
        (
            Span::styled(
                "✖",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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
        QualityStatus::Good => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        QualityStatus::Warning => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        QualityStatus::Reject => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let blur_style = match quality.blur.level {
        BlurLevel::Low => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        BlurLevel::Medium => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        BlurLevel::High => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let quality_style = if quality.overall_quality >= 0.75 {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if quality.overall_quality >= 0.50 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };

    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "FACE QUALITY",
                Style::default().add_modifier(Modifier::BOLD),
            ),
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
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
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
        left_lines.push(Line::from(Span::styled(
            quality.resolution_display(),
            val_style,
        )));
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
        right_lines.push(Line::from(Span::styled(
            quality.quality_display(),
            quality_style,
        )));
        if spaced {
            right_lines.push(Line::from(""));
        }
        right_lines.push(Line::from(Span::styled("Status:", key_style)));
        right_lines.push(Line::from(Span::styled(
            quality.status_display(),
            status_style,
        )));

        frame.render_widget(Paragraph::new(left_lines), cols[0]);
        frame.render_widget(Paragraph::new(right_lines), cols[1]);
    }

    frame.render_widget(block, area);
}

fn render_discovery(frame: &mut Frame, area: Rect, app: &App) {
    let is_failure = app.discovery_error.is_some() || app.discovery_request_status == "FAILED";

    let border_color = if is_failure { Color::Red } else { Color::Cyan };

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(border_color)),
            Span::styled("DISCOVERY", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);

    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let req_style = if is_failure {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    };

    if inner.height >= 14 {
        // Taller viewports: single vertical layout with blank line separators
        let mut lines = vec![
            Line::from(Span::styled(
                "DISCOVERY",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("Provider:", key_style)),
            Line::from(Span::styled(&app.discovery_provider, val_style)),
            Line::from(""),
            Line::from(Span::styled("Request:", key_style)),
            Line::from(Span::styled(&app.discovery_request_status, req_style)),
            Line::from(""),
            Line::from(Span::styled("Candidates:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_raw_count),
                val_style,
            )),
            Line::from(""),
            Line::from(Span::styled("Unique:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_unique_count),
                val_style,
            )),
        ];

        if let Some(err) = &app.discovery_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "SEARCH FAILURE:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                err.as_str(),
                Style::default().fg(Color::LightRed),
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    } else if let Some(err) = &app.discovery_error {
        // Standard viewports with failure: two-column metrics + prominent failure banner
        let rows = Layout::vertical([
            Constraint::Length(5), // Discovery metrics
            Constraint::Fill(1),   // Failure banner
        ])
        .split(inner);

        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);

        let left_lines = vec![
            Line::from(Span::styled(
                "DISCOVERY",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("Provider:", key_style)),
            Line::from(Span::styled(&app.discovery_provider, val_style)),
            Line::from(Span::styled("Request:", key_style)),
            Line::from(Span::styled(&app.discovery_request_status, req_style)),
        ];

        let right_lines = vec![
            Line::from(""),
            Line::from(Span::styled("Candidates:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_raw_count),
                val_style,
            )),
            Line::from(Span::styled("Unique:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_unique_count),
                val_style,
            )),
        ];

        let err_lines = vec![
            Line::from(Span::styled(
                "SEARCH FAILURE:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                err.as_str(),
                Style::default().fg(Color::LightRed),
            )),
        ];

        frame.render_widget(Paragraph::new(left_lines), cols[0]);
        frame.render_widget(Paragraph::new(right_lines), cols[1]);
        frame.render_widget(Paragraph::new(err_lines), rows[1]);
    } else {
        // Standard viewports without failure: clean vertical layout matching prompt format
        let lines = vec![
            Line::from(Span::styled(
                "DISCOVERY",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled("Provider:", key_style)),
            Line::from(Span::styled(&app.discovery_provider, val_style)),
            Line::from(Span::styled("Request:", key_style)),
            Line::from(Span::styled(&app.discovery_request_status, req_style)),
            Line::from(Span::styled("Candidates:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_raw_count),
                val_style,
            )),
            Line::from(Span::styled("Unique:", key_style)),
            Line::from(Span::styled(
                format!("{}", app.discovery_unique_count),
                val_style,
            )),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }

    frame.render_widget(block, area);
}

fn render_candidate_verification(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "CANDIDATE VERIFICATION & RANKING",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);

    let ranked_list: Vec<RankedCandidate> = if !app.ranked_candidates.is_empty() {
        app.ranked_candidates.clone()
    } else if !app.verified_candidates.is_empty() {
        tekmerion_verification::CandidateRanker::new().rank_results(app.verified_candidates.clone())
    } else {
        Vec::new()
    };

    if ranked_list.is_empty() {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No candidates verified yet.",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "  Advance through pipeline stages or press [V] to verify candidates.",
                Style::default().fg(Color::DarkGray),
            )),
        ]);
        frame.render_widget(placeholder, inner);
        frame.render_widget(block, area);
        return;
    }

    let selected_idx = app
        .selected_candidate
        .min(ranked_list.len().saturating_sub(1));

    if inner.height >= 9 {
        let detail_height = if inner.height >= 11 { 5 } else { 4 };
        let rows = Layout::vertical([
            Constraint::Length(1),             // summary header
            Constraint::Fill(1),               // candidate list with column headers
            Constraint::Length(detail_height), // selected candidate detail box
        ])
        .split(inner);

        // 1. Summary Header
        let verified_count = ranked_list
            .iter()
            .filter(|c| c.status() == VerificationStatus::Verified)
            .count();
        let total_count = ranked_list.len();
        let summary_line = Line::from(vec![
            Span::styled(" Candidates: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{total_count}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Verified: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{verified_count}"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │  Threshold: ", Style::default().fg(Color::DarkGray)),
            Span::styled("≥ 0.75", Style::default().fg(Color::Cyan)),
            Span::styled("  │  Use ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "↑/↓",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to inspect", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(summary_line), rows[0]);

        // 2. Candidate Table with explicit columns:
        // RANK, SOURCE, SIMILARITY, QUALITY, SCORE, STATUS
        let mut list_lines: Vec<Line> = Vec::new();

        let col_header = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "RANK ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "SOURCE   ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "SIMILARITY ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "QUALITY ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "SCORE  ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "STATUS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        list_lines.push(col_header);

        for (idx, cand) in ranked_list.iter().enumerate() {
            let is_selected = idx == selected_idx;
            let cursor = if is_selected {
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("  ")
            };

            let rank_span = Span::styled(
                format!("{:<4}", format!("#{}", cand.rank)),
                if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            );

            let domain = cand.source();
            let domain_display = if domain.len() > 8 {
                format!("{:<8}", &domain[..8])
            } else {
                format!("{:<8}", domain)
            };
            let source_span = Span::styled(
                format!("{} ", domain_display),
                if is_selected {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                },
            );

            let sim_span = Span::styled(
                format!("{:<6}", format!("{:.2}", cand.face_similarity)),
                if is_selected {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                },
            );

            let qual_span = Span::styled(
                format!("{:<6}", format!("{:.2}", cand.quality_score)),
                if is_selected {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::Gray)
                },
            );

            let score_span = Span::styled(
                format!("{:<6}", format!("{:.2}", cand.ranking_score)),
                if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow)
                },
            );

            let (status_text, status_style) = match cand.status() {
                VerificationStatus::Verified => (
                    "VERIFIED",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                VerificationStatus::BelowThreshold => (
                    "BELOW THRESHOLD",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                VerificationStatus::NoFace => (
                    "NO FACE",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
                VerificationStatus::Error => (
                    "ERROR",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            };
            let status_span = Span::styled(status_text, status_style);

            list_lines.push(Line::from(vec![
                cursor,
                rank_span,
                source_span,
                sim_span,
                qual_span,
                score_span,
                status_span,
            ]));
        }

        frame.render_widget(Paragraph::new(list_lines), rows[1]);

        // 3. Selected Detail Box
        let selected_cand = &ranked_list[selected_idx];
        let detail_block = Block::default()
            .borders(Borders::TOP)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Line::from(vec![
                Span::styled(" Selected Candidate #", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("{}", selected_cand.rank),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
            ]));

        let key_style = Style::default().fg(Color::DarkGray);
        let val_style = Style::default().fg(Color::White);

        let hash_str = selected_cand
            .verification
            .candidate_image_hash
            .as_deref()
            .unwrap_or("--");
        let hash_display = if hash_str.len() > 16 {
            &hash_str[..16]
        } else {
            hash_str
        };

        let status_style = match selected_cand.status() {
            VerificationStatus::Verified => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            VerificationStatus::BelowThreshold => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            VerificationStatus::NoFace => Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
            VerificationStatus::Error => {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            }
        };

        let face_str = selected_cand
            .verification
            .matched_face_index
            .map(|i| format!("Face #{i}"))
            .unwrap_or_else(|| "Face --".to_string());

        let domain_str = selected_cand.candidate().domain.as_str();

        let detail_lines = vec![
            Line::from(vec![
                Span::styled("RANK: ", key_style),
                Span::styled(
                    format!("#{:<3}", selected_cand.rank),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("SCORE: ", key_style),
                Span::styled(
                    format!("{:<6.4} ", selected_cand.ranking_score),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("QUALITY: ", key_style),
                Span::styled(format!("{:<4.2}", selected_cand.quality_score), val_style),
            ]),
            Line::from(vec![
                Span::styled("SOURCE: ", key_style),
                Span::styled(format!("{:<20} ", domain_str), val_style),
                Span::styled("STATUS: ", key_style),
                Span::styled(selected_cand.status().label(), status_style),
            ]),
            Line::from(vec![
                Span::styled("SIMILARITY: ", key_style),
                Span::styled(
                    format!("{:.2} (Sim:) ", selected_cand.face_similarity),
                    val_style,
                ),
                Span::styled(
                    format!("{:<8} ", face_str),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(hash_display, Style::default().fg(Color::Cyan)),
            ]),
        ];

        let detail_inner = detail_block.inner(rows[2]);
        frame.render_widget(detail_block, rows[2]);
        frame.render_widget(Paragraph::new(detail_lines), detail_inner);
    } else {
        // Compact view for small terminals
        let mut list_lines: Vec<Line> = Vec::new();
        list_lines.push(Line::from(vec![
            Span::styled("RANK ", Style::default().fg(Color::Cyan)),
            Span::styled("SOURCE     ", Style::default().fg(Color::Cyan)),
            Span::styled("SIM   ", Style::default().fg(Color::Cyan)),
            Span::styled("QUAL  ", Style::default().fg(Color::Cyan)),
            Span::styled("SCORE  ", Style::default().fg(Color::Cyan)),
            Span::styled("STATUS", Style::default().fg(Color::Cyan)),
        ]));

        for (idx, cand) in ranked_list.iter().enumerate() {
            let is_selected = idx == selected_idx;
            let cursor = if is_selected { "▶ " } else { "  " };
            let (status_str, status_color) = match cand.status() {
                VerificationStatus::Verified => ("VERIFIED", Color::Green),
                VerificationStatus::BelowThreshold => ("BELOW_TH", Color::Yellow),
                VerificationStatus::NoFace => ("NO_FACE", Color::DarkGray),
                VerificationStatus::Error => ("ERROR", Color::Red),
            };

            list_lines.push(Line::from(vec![
                Span::styled(cursor, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("#{:<3} ", cand.rank),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:<10} ", &cand.source()[..cand.source().len().min(10)]),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{:<5.2} ", cand.face_similarity),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{:<5.2} ", cand.quality_score),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{:<6.2} ", cand.ranking_score),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    status_str,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(list_lines), inner);
    }

    frame.render_widget(block, area);
}

fn render_evidence_tree(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "EVIDENCE TREE",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if app.status == AppStatus::Tampered {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Cyan)
        });

    let key_style = Style::default().fg(Color::White);
    let check_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let root_label_style = Style::default().fg(Color::DarkGray);
    let root_val_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let is_tampered = app.status == AppStatus::Tampered;

    let content_leaf_line = if is_tampered {
        Line::from(vec![
            Span::styled(format!("{:<15}", "CONTENT"), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("  [TAMPER DETECTED - title modified]", Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{:<15}", "CONTENT"), key_style),
            Span::styled("✓", check_style),
            Span::styled("  [Canonical JSON payload digest]", Style::default().fg(Color::DarkGray)),
        ])
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "FIVE-LEAF CRYPTOGRAPHIC ANCHOR",
            Style::default()
                .fg(if is_tampered { Color::Red } else { Color::Cyan })
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:<15}", "IMAGE"), key_style),
            Span::styled("✓", check_style),
            Span::styled("  [SHA-256 binary image digest]", Style::default().fg(Color::DarkGray)),
        ]),
        content_leaf_line,
        Line::from(vec![
            Span::styled(format!("{:<15}", "METADATA"), key_style),
            Span::styled("✓", check_style),
            Span::styled("  [Source URL & timestamp digest]", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "FACE"), key_style),
            Span::styled("✓", check_style),
            Span::styled("  [Biometric embedding & quality]", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(format!("{:<15}", "PROVENANCE"), key_style),
            Span::styled("✓", check_style),
            Span::styled("  [Run ID & provider attestation]", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
    ];

    if is_tampered {
        lines.push(Line::from(vec![
            Span::styled("ROOT (LOCAL): ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(app.evidence_root.as_str(), Style::default().fg(Color::Red)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("ROOT (CHAIN): ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(app.chain_root.as_str(), Style::default().fg(Color::Green)),
            Span::styled("  [MISMATCH ✗]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(Span::styled("ROOT", root_label_style)));
        lines.push(Line::from(Span::styled(app.evidence_root.as_str(), root_val_style)));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_input_stage(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "INPUT INGESTION",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(inner);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "INPUT STAGE ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ Status: ", key_style),
        Span::styled("RUNNING", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" │ SHA-256 Checksum", Style::default().fg(Color::Gray)),
    ]));

    // Web KPI badge row
    lines.push(Line::from(vec![
        Span::styled(" [ 1920x1080 ] ", Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 35, 45))),
        Span::styled(" [ JPEG ] ", Style::default().fg(Color::Green).bg(Color::Rgb(15, 40, 20))),
        Span::styled(" [ SHA-256 ] ", Style::default().fg(Color::Yellow).bg(Color::Rgb(45, 40, 15))),
        Span::styled(" [ ARMED ] ", Style::default().fg(Color::White).bg(Color::Rgb(30, 40, 50))),
    ]));

    if inner.height >= 14 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("WHY THIS MATTERS (EXPLANATION):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Ingests the query face photo, calculates SHA-256 digest, and validates dimensions for biometric analysis.",
                Style::default().fg(Color::Rgb(210, 215, 225)),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("File: ", key_style),
        Span::styled(&app.input_image_name, val_style),
        Span::styled("  │  Resolution: ", key_style),
        Span::styled(&app.input_image_resolution, val_style),
    ]));

    let hash_snip = if app.input_image_hash.len() > 20 {
        &app.input_image_hash[..20]
    } else {
        &app.input_image_hash
    };
    lines.push(Line::from(vec![
        Span::styled("Image SHA256: ", key_style),
        Span::styled(hash_snip, Style::default().fg(Color::Cyan)),
        Span::styled("...", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Model: ", key_style),
        Span::styled("insightface-arcface-r100", val_style),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Readiness: ", key_style),
        Span::styled("✓ Format valid  ✓ Pipeline armed", Style::default().fg(Color::Green)),
    ]));

    if area.height >= 14 {
        lines.push(Line::from(vec![
            Span::styled("Custom: ", key_style),
            Span::styled("cargo run -p tekmerion-tui -- <image_path>", Style::default().fg(Color::DarkGray)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);

    let percent = app.progress_percent();
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
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

fn render_blockchain_stage(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "BLOCKCHAIN ANCHORING",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(inner);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "ON-CHAIN ANCHOR ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ Proof of Existence", Style::default().fg(Color::Gray)),
    ]));

    // Web KPI badge row
    lines.push(Line::from(vec![
        Span::styled(" [ SEPOLIA ] ", Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 35, 45))),
        Span::styled(" [ FINALIZED ] ", Style::default().fg(Color::Green).bg(Color::Rgb(15, 40, 20))),
        Span::styled(" [ SMART CONTRACT ] ", Style::default().fg(Color::Yellow).bg(Color::Rgb(45, 40, 15))),
    ]));

    if inner.height >= 14 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("WHY THIS MATTERS (EXPLANATION):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Anchors the Evidence Merkle Root into an immutable smart contract for permanent proof of existence.",
                Style::default().fg(Color::Rgb(210, 215, 225)),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("Network: ", key_style),
        Span::styled(&app.blockchain_network, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]));

    let contract_snip = if app.blockchain_contract.len() > 18 {
        format!("{}...", &app.blockchain_contract[..18])
    } else {
        app.blockchain_contract.clone()
    };
    lines.push(Line::from(vec![
        Span::styled("Contract: ", key_style),
        Span::styled(contract_snip, val_style),
    ]));

    lines.push(Line::from(vec![
        Span::styled("TX Hash: ", key_style),
        if app.tx_hash == "--" {
            Span::styled("--", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(format!("{}...", &app.tx_hash[..18.min(app.tx_hash.len())]), Style::default().fg(Color::Cyan))
        },
    ]));

    let block_str = if app.blockchain_block > 0 {
        format!("#{}", app.blockchain_block)
    } else {
        "--".to_string()
    };
    let conf_str = if app.blockchain_confirmations > 0 {
        format!("{} blocks (Finalized)", app.blockchain_confirmations)
    } else {
        "--".to_string()
    };

    lines.push(Line::from(vec![
        Span::styled("Block: ", key_style),
        Span::styled(block_str, val_style),
        Span::styled("  │  Conf: ", key_style),
        Span::styled(conf_str, Style::default().fg(Color::Green)),
    ]));

    frame.render_widget(Paragraph::new(lines), rows[0]);

    let percent = app.progress_percent();
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
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

fn render_final_verify_stage(frame: &mut Frame, area: Rect, app: &App) {
    let is_tampered = app.status == AppStatus::Tampered;
    let border_color = if is_tampered { Color::Red } else { Color::Green };

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(border_color)),
            Span::styled(
                "FINAL ON-CHAIN VERIFICATION",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let rows = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(inner);

    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "VERIFY ON-CHAIN ",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ Cryptographic Anchor Comparison", Style::default().fg(Color::Gray)),
    ]));

    if is_tampered {
        lines.push(Line::from(vec![
            Span::styled(" [ TAMPER DETECTED ] ", Style::default().fg(Color::Red).bg(Color::Rgb(45, 15, 15)).add_modifier(Modifier::BOLD)),
            Span::styled(" [ HASH MISMATCH ] ", Style::default().fg(Color::Yellow).bg(Color::Rgb(45, 40, 15))),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(" [ ROOT MATCH ] ", Style::default().fg(Color::Green).bg(Color::Rgb(15, 40, 20)).add_modifier(Modifier::BOLD)),
            Span::styled(" [ 0 MISMATCHES ] ", Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 35, 45))),
            Span::styled(" [ AUDIT VERIFIED ] ", Style::default().fg(Color::White).bg(Color::Rgb(30, 40, 50))),
        ]));
    }

    if inner.height >= 14 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("WHY THIS MATTERS (EXPLANATION):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Reads back the anchor record from the smart contract and verifies byte-for-byte that the local evidence matches.",
                Style::default().fg(Color::Rgb(210, 215, 225)),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("Contract Query: ", key_style),
        Span::styled("eth_call -> getAnchorRecord(root)", val_style),
    ]));

    let chain_snip = if app.chain_root == "--" {
        if app.evidence_root == "--" {
            "--".to_string()
        } else {
            format!("{}...", &app.evidence_root[..16.min(app.evidence_root.len())])
        }
    } else {
        format!("{}...", &app.chain_root[..16.min(app.chain_root.len())])
    };

    let local_snip = if app.evidence_root == "--" {
        "--".to_string()
    } else {
        format!("{}...", &app.evidence_root[..16.min(app.evidence_root.len())])
    };

    lines.push(Line::from(vec![
        Span::styled("On-Chain Root: ", key_style),
        Span::styled(chain_snip, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));

    if is_tampered {
        lines.push(Line::from(vec![
            Span::styled("Local Root:    ", key_style),
            Span::styled(local_snip, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("  [MISMATCH ✗]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
        if let Some(leaf) = &app.tampered_leaf {
            lines.push(Line::from(vec![
                Span::styled("Tampered Leaf: ", Style::default().fg(Color::Yellow)),
                Span::styled(leaf, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(" (field: {})", app.tampered_field.as_deref().unwrap_or("title")),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
        lines.push(Line::from(vec![
            Span::styled("Integrity:     ", key_style),
            Span::styled("FAILED (Cryptographic tamper detected)", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Local Root:    ", key_style),
            Span::styled(local_snip, Style::default().fg(Color::Cyan)),
            Span::styled("  [MATCH ✓]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Integrity:     ", key_style),
            Span::styled("100% Cryptographic Match ✓", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);

    let percent = app.progress_percent();
    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(border_color)
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

fn render_pipeline_overview(frame: &mut Frame, area: Rect, app: &App) {
    let (status_badge, status_style) = match app.status {
        AppStatus::Idle => (
            " IDLE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
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
        "tampered" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        _ => Style::default().fg(Color::DarkGray),
    };

    let selected = if app.candidate_count == 0 {
        "none".to_string()
    } else {
        format!("{}/{}", app.selected_candidate + 1, app.candidate_count)
    };

    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

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

    let mut lines = vec![
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

    if inner.height >= 12 {
        if app.status == AppStatus::Idle {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("▌ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled("READY TO RUN: Press [ENTER] to start pipeline | [?] for Guide", Style::default().fg(Color::Cyan)),
            ]));
        } else if app.status == AppStatus::Completed {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("▌ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled("Verification Complete: Evidence anchored & on-chain verified", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
        } else if app.status == AppStatus::Tampered {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("▌ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled("Tamper Detected: Cryptographic integrity check failed", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]));
        }
    }

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

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    if app.current == Some(Stage::Input) {
        render_input_stage(frame, area, app);
        return;
    }

    if app.current == Some(Stage::Face) {
        render_face_quality(frame, area, app);
        return;
    }

    if app.current == Some(Stage::Discovery) {
        render_discovery(frame, area, app);
        return;
    }

    if app.current == Some(Stage::Verify) {
        render_candidate_verification(frame, area, app);
        return;
    }

    if app.current == Some(Stage::Evidence) {
        render_evidence_tree(frame, area, app);
        return;
    }

    if app.current == Some(Stage::Blockchain) {
        render_blockchain_stage(frame, area, app);
        return;
    }

    if app.current == Some(Stage::FinalVerify) {
        render_final_verify_stage(frame, area, app);
        return;
    }

    render_pipeline_overview(frame, area, app);
}

fn render_tab_evidence(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "EVIDENCE & MERKLE TREE DEEP DIVE",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    let key_style = Style::default().fg(Color::DarkGray);
    let val_style = Style::default().fg(Color::White);

    let rows = Layout::vertical([
        Constraint::Length(2), // Intro
        Constraint::Fill(1),   // Merkle Tree + Record details
        Constraint::Length(2), // Footnote explainer
    ])
    .split(inner);

    let intro = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Schema: ", key_style),
            Span::styled("tekmerion.evidence.v1", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("  │  Algorithm: ", key_style),
            Span::styled("SHA-256 Binary Merkle Tree", val_style),
            Span::styled("  │  Leaves: ", key_style),
            Span::styled("5 Proof Nodes", Style::default().fg(Color::Yellow)),
        ]),
    ]);
    frame.render_widget(intro, rows[0]);

    let cols = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(rows[1]);

    let is_tampered = app.status == AppStatus::Tampered;

    let mut tree_lines = vec![
        Line::from(Span::styled("MERKLE TREE TOPOLOGY", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
    ];

    if is_tampered {
        tree_lines.push(Line::from(vec![
            Span::styled("Local Root: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(
                if app.evidence_root == "--" { "--" } else { &app.evidence_root[..app.evidence_root.len().min(24)] },
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [MISMATCH ✗]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
        tree_lines.push(Line::from(vec![
            Span::styled("Chain Root: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(
                if app.chain_root == "--" { "--" } else { &app.chain_root[..app.chain_root.len().min(24)] },
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [ANCHORED ✓]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]));
        tree_lines.push(Line::from("├── Leaf 0 [IMAGE]      SHA-256 Digest of candidate image ✓"));
        tree_lines.push(Line::from(vec![
            Span::styled("├── Leaf 1 [CONTENT]    Canonical title & snippet text ", Style::default().fg(Color::Yellow)),
            Span::styled("✗ [TAMPERED - title altered]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]));
        tree_lines.push(Line::from("├── Leaf 2 [METADATA]   Domain, URL & retrieval timestamp ✓"));
        tree_lines.push(Line::from("├── Leaf 3 [FACE]       Biometric embedding similarity & quality ✓"));
        tree_lines.push(Line::from("└── Leaf 4 [PROVENANCE] Run ID, provider & schema version ✓"));
    } else {
        tree_lines.push(Line::from(vec![
            Span::styled("Root Hash: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(
                if app.evidence_root == "--" { "--" } else { &app.evidence_root[..app.evidence_root.len().min(24)] },
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        tree_lines.push(Line::from("├── Leaf 0 [IMAGE]      SHA-256 Digest of candidate image"));
        tree_lines.push(Line::from("├── Leaf 1 [CONTENT]    Canonical title & snippet text"));
        tree_lines.push(Line::from("├── Leaf 2 [METADATA]   Domain, URL & retrieval timestamp"));
        tree_lines.push(Line::from("├── Leaf 3 [FACE]       Biometric embedding similarity & quality"));
        tree_lines.push(Line::from("└── Leaf 4 [PROVENANCE] Run ID, provider & schema version"));
    }
    frame.render_widget(Paragraph::new(tree_lines), cols[0]);

    let title_display = if is_tampered {
        Span::styled(
            "Modified photograph [UNAUTHORIZED ALTERATION]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("Original photograph", val_style)
    };

    let status_display = if is_tampered {
        Span::styled("TAMPER DETECTED ✗ (Hash Mismatch)", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else if app.evidence_root == "--" {
        Span::styled("Pending assembly", val_style)
    } else {
        Span::styled("Assembled & Validated ✓", Style::default().fg(Color::Green))
    };

    let record_lines = vec![
        Line::from(Span::styled("CANONICAL RECORD", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("Run ID: ", key_style), Span::styled("demo-run-001", val_style)]),
        Line::from(vec![Span::styled("Title: ", key_style), title_display]),
        Line::from(vec![Span::styled("Model: ", key_style), Span::styled("insightface-arcface-r100", val_style)]),
        Line::from(vec![Span::styled("Format: ", key_style), Span::styled("RFC 8785 Canonical JSON", val_style)]),
        Line::from(vec![Span::styled("Auditability: ", key_style), Span::styled("Zero-Knowledge Merkle Path", Style::default().fg(Color::Green))]),
        Line::from(vec![Span::styled("Status: ", key_style), status_display]),
    ];
    frame.render_widget(Paragraph::new(record_lines), cols[1]);

    let footnote = Paragraph::new(Line::from(vec![
        Span::styled("▶ WHY THIS MATTERS: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(
            "Merkle proofs allow third-party auditors to verify any single field without seeing the rest of the private dataset.",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(footnote, rows[2]);

    frame.render_widget(block, area);
}

fn render_tab_candidates(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "CANDIDATE INSPECTOR & COMPARISON",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);

    let ranked_list: Vec<RankedCandidate> = if !app.ranked_candidates.is_empty() {
        app.ranked_candidates.clone()
    } else if !app.verified_candidates.is_empty() {
        tekmerion_verification::CandidateRanker::new().rank_results(app.verified_candidates.clone())
    } else {
        Vec::new()
    };

    if ranked_list.is_empty() {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No candidates discovered yet.",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "  Press [ENTER] to start and [V] to advance to Discovery & Verify stages.",
                Style::default().fg(Color::DarkGray),
            )),
        ]);
        frame.render_widget(placeholder, inner);
        frame.render_widget(block, area);
        return;
    }

    let selected_idx = app.selected_candidate.min(ranked_list.len().saturating_sub(1));

    let rows = Layout::vertical([
        Constraint::Length(1), // summary
        Constraint::Fill(1),   // table
        Constraint::Length(4), // selected candidate details
    ])
    .split(inner);

    let summary = Line::from(vec![
        Span::styled("Total Candidates: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", ranked_list.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled("  │  Threshold: ", Style::default().fg(Color::DarkGray)),
        Span::styled("≥ 0.75", Style::default().fg(Color::Cyan)),
        Span::styled("  │  Use ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑/↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" to inspect candidates", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(summary), rows[0]);

    let mut table_lines = Vec::new();
    table_lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("RANK ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("SOURCE DOMAIN       ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("SIMILARITY ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("QUALITY ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("SCORE  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("STATUS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]));

    for (idx, cand) in ranked_list.iter().enumerate() {
        let is_selected = idx == selected_idx;
        let cursor = if is_selected {
            Span::styled("▶ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::raw("  ")
        };

        let rank_span = Span::styled(format!("#{:<4}", cand.rank), if is_selected { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) });
        let domain_span = Span::styled(format!("{:<20}", cand.source()), if is_selected { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) });
        let sim_span = Span::styled(format!("{:<11.2}", cand.face_similarity), if is_selected { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) });
        let qual_span = Span::styled(format!("{:<8.2}", cand.quality_score), if is_selected { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) });
        let score_span = Span::styled(format!("{:<7.2}", cand.ranking_score), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        let (status_text, status_style) = match cand.status() {
            VerificationStatus::Verified => ("VERIFIED", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            VerificationStatus::BelowThreshold => ("BELOW THRESHOLD", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            VerificationStatus::NoFace => ("NO FACE", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            VerificationStatus::Error => ("ERROR", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        };

        table_lines.push(Line::from(vec![
            cursor,
            rank_span,
            domain_span,
            sim_span,
            qual_span,
            score_span,
            Span::styled(status_text, status_style),
        ]));
    }
    frame.render_widget(Paragraph::new(table_lines), rows[1]);

    let sel = &ranked_list[selected_idx];
    let sel_lines = vec![
        Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("#{} {}", sel.rank, sel.source()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  │  URL: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sel.candidate().url.as_str(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sel.candidate().title.as_deref().unwrap_or("None"), Style::default().fg(Color::White)),
            Span::styled("  │  Image Hash: ", Style::default().fg(Color::DarkGray)),
            Span::styled(sel.verification.candidate_image_hash.as_deref().unwrap_or("--"), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Scoring Formula: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Score = (Similarity × 0.7) + (Quality × 0.3)  │  Threshold: ≥ 0.75 required for match verification", Style::default().fg(Color::Gray)),
        ]),
    ];
    frame.render_widget(Paragraph::new(sel_lines).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray))), rows[2]);

    frame.render_widget(block, area);
}

fn render_tab_guide(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "SYSTEM GUIDE & ARCHITECTURE",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);

    let lines = vec![
        Line::from(Span::styled("HOW TEKMERION WORKS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("Tekmerion is an Evidence Intelligence Engine that verifies face image matches and provides tamper-proof cryptographic audit trails."),
        Line::from(""),
        Line::from(vec![
            Span::styled("1. FACE ANALYSIS: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Extracts 512-dimensional facial embedding vectors (ArcFace-R100) and scores blur, pose, and resolution.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("2. CANDIDATE DISCOVERY: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Performs reverse image searches across public sources and isolates unique candidate web URLs.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("3. BIOMETRIC VERIFICATION: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Computes cosine similarity between query and candidate face embeddings. Threshold is ≥ 0.75.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("4. MERKLE EVIDENCE ASSEMBLY: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Combines image hash, URL, metadata, and scores into a cryptographic binary Merkle tree (SHA-256).", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("5. BLOCKCHAIN ANCHORING: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Anchors the Merkle Root hash to a smart contract. Creates an immutable public proof of existence.", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("6. ZERO-KNOWLEDGE PRIVACY: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("No biometric facial images are EVER stored on the public blockchain. Only 32-byte cryptographic hashes are anchored.", Style::default().fg(Color::White)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
    frame.render_widget(block, area);
}

fn render_help_modal(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(72, 80, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled("TEKMERION HELP & CONTROLS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);

    let key_badge = |k: &'static str| Span::styled(format!(" [{k}] "), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD));

    let lines = vec![
        Line::from(Span::styled("KEYBOARD SHORTCUTS:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            key_badge("ENTER"),
            Span::styled(" Start the 7-stage verification pipeline", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("V"),
            Span::styled(" Advance / verify the current stage", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("T"),
            Span::styled(" Flag a tamper event (simulates cryptographic tampering)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("R"),
            Span::styled(" Reset pipeline to idle state", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("↑/↓"),
            Span::styled(" Navigate and select candidate to inspect", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("TAB"),
            Span::styled(" Cycle view tabs (Flow → Evidence → Candidates → Guide)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("1-4"),
            Span::styled(" Directly switch to view tab: 1:Flow, 2:Evidence, 3:Candidates, 4:Guide", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("? / H"),
            Span::styled(" Toggle this help window", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("ESC"),
            Span::styled(" Close this help window", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            key_badge("Q"),
            Span::styled(" Quit Tekmerion TUI", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("PIPELINE STATUS GLYPHS:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled(" ▶ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("Active stage currently in progress", Style::default().fg(Color::White)),
            Span::styled("   │   ", Style::default().fg(Color::DarkGray)),
            Span::styled(" ✓ ", Style::default().fg(Color::Green)),
            Span::styled("Stage completed successfully", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" ○ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Pending stage", Style::default().fg(Color::White)),
            Span::styled("                        │   ", Style::default().fg(Color::DarkGray)),
            Span::styled(" ✖ ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("Tamper detected or error", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press [ESC] or [?] to dismiss this help window", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
    frame.render_widget(block, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn render_events(frame: &mut Frame, area: Rect, app: &App) {
    let count_label = format!(" [{}/{} events] ", app.events.len(), 8);
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "LIVE AUDIT STREAM",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(count_label, Style::default().fg(Color::DarkGray)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.events.is_empty() {
        let placeholder = Paragraph::new(Line::from(vec![
            Span::styled("  [ STANDBY ] ", Style::default().fg(Color::DarkGray).bg(Color::Rgb(20, 25, 30))),
            Span::styled(
                "No events recorded yet. Press [ENTER] to launch verification pipeline.",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
        frame.render_widget(placeholder, inner);
        return;
    }

    let visible_lines = inner.height as usize;
    let skip = app.events.len().saturating_sub(visible_lines);

    let lines: Vec<Line> = app
        .events
        .iter()
        .skip(skip)
        .map(|event| {
            let (tag_text, tag_fg, tag_bg) = if event.contains("verified") || event.contains("100% match") {
                (" PASS ", Color::Green, Color::Rgb(15, 40, 20))
            } else if event.contains("complete") {
                (" DONE ", Color::Green, Color::Rgb(15, 40, 20))
            } else if event.contains("started") || event.contains("Loaded") {
                (" FLOW ", Color::Cyan, Color::Rgb(20, 35, 45))
            } else if event.contains("Tamper") || event.contains("mismatch") {
                (" WARN ", Color::Red, Color::Rgb(45, 15, 15))
            } else if event.contains("reset") {
                (" SYNC ", Color::Yellow, Color::Rgb(45, 40, 15))
            } else if event.contains("anchored") || event.contains("blockchain") {
                (" ANCH ", Color::Yellow, Color::Rgb(45, 40, 15))
            } else {
                (" LOGS ", Color::White, Color::Rgb(30, 35, 40))
            };

            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    tag_text,
                    Style::default().fg(tag_fg).bg(tag_bg).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(event.as_str(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let mut spans = vec![
        Span::styled(
            " [ENTER] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Start ", Style::default().fg(Color::White)),
        Span::styled(
            " [V] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Advance ", Style::default().fg(Color::White)),
        Span::styled(
            " [T] ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Tamper ", Style::default().fg(Color::White)),
        Span::styled(
            " [R] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Reset ", Style::default().fg(Color::White)),
        Span::styled(
            " [↑/↓] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(100, 150, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Select ", Style::default().fg(Color::White)),
    ];

    if area.width >= 105 {
        spans.push(Span::styled(
            " [Tab] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("Views ", Style::default().fg(Color::White)));
    }

    if area.width >= 92 {
        spans.push(Span::styled(
            " [?] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("Help ", Style::default().fg(Color::White)));
    }

    spans.push(Span::styled(
        " [Q] ",
        Style::default()
            .fg(Color::White)
            .bg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled("Quit", Style::default().fg(Color::White)));

    let footer = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}
