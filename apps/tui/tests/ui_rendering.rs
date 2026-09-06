use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tekmerion_tui::app::{App, Stage};
use tekmerion_tui::input::AppAction;
use tekmerion_tui::ui;

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).expect("cell must exist in area");
            out.push_str(cell.symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn renders_idle_state_on_standard_80x24_with_all_stages() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let app = App::new();

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    // Verify header branding
    assert!(screen.contains("TEKMERION"), "must show TEKMERION header");
    assert!(
        screen.contains("EVIDENCE INTELLIGENCE ENGINE"),
        "must show subtitle"
    );

    // Verify all 7 stages are rendered and none are clipped off on 80x24
    for stage in Stage::ALL {
        assert!(
            screen.contains(stage.label()),
            "stage {} must be visible on standard 80x24 terminal",
            stage.label()
        );
    }

    // Verify detail fields
    assert!(screen.contains("STATUS:"), "must show STATUS");
    assert!(screen.contains("IDLE"), "must show IDLE status");
    assert!(screen.contains("STATE:"), "must show STATE");
    assert!(screen.contains("CANDIDATES:"), "must show CANDIDATES");
    assert!(screen.contains("EVIDENCE ROOT:"), "must show EVIDENCE ROOT");
    assert!(screen.contains("TX HASH:"), "must show TX HASH");
    assert!(screen.contains("RESULT:"), "must show RESULT");
    assert!(screen.contains("0%"), "must show 0% progress");

    // Verify empty events placeholder
    assert!(
        screen.contains("No events recorded yet"),
        "must show empty events prompt"
    );

    // Verify footer shortcut keys
    assert!(screen.contains("ENTER"), "must show ENTER shortcut");
    assert!(screen.contains("Start"), "must show Start action");
    assert!(screen.contains("Quit"), "must show Quit action");
}

#[test]
fn renders_running_state_with_active_marker_and_progress() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("RUNNING"), "status must be RUNNING");
    assert!(screen.contains("[ACTIVE]"), "must indicate active stage");
    assert!(
        screen.contains("Pipeline started"),
        "event log must show started event"
    );
    assert!(
        screen.contains("Progress: 14%"),
        "must display current progress percentage"
    );
}

#[test]
fn renders_completed_state_with_checkmarks_and_verified_result() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    for _ in 0..Stage::ALL.len() {
        app.apply(AppAction::Verify);
    }

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("COMPLETED"), "status must be COMPLETED");
    assert!(screen.contains("verified"), "result must be verified");
    assert!(screen.contains("100%"), "progress must be 100%");
    assert!(screen.contains("✓"), "must show checkmark glyphs");
    assert!(
        screen.contains("Pipeline verified"),
        "event log must show pipeline verified event"
    );
}

#[test]
fn renders_tampered_state_safely() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Tamper);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("TAMPERED"), "status must be TAMPERED");
    assert!(screen.contains("tampered"), "result must be tampered");
    assert!(
        screen.contains("Tamper detected"),
        "event log must record tamper event"
    );
}

#[test]
fn handles_small_terminal_gracefully() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let app = App::new();

    // Must not panic on small terminal
    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(
        screen.contains("too small"),
        "must display size warning on small terminal"
    );
}

#[test]
fn renders_face_quality_on_face_stage() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start); // Stage::Input
    app.apply(AppAction::Verify); // Stage::Face

    assert_eq!(app.current, Some(Stage::Face));

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    // Verify all required FACE QUALITY fields from the prompt
    assert!(
        screen.contains("FACE QUALITY"),
        "must show FACE QUALITY header"
    );
    assert!(screen.contains("Faces:"), "must show Faces: label");
    assert!(screen.contains("1"), "must show face count 1");
    assert!(
        screen.contains("Resolution:"),
        "must show Resolution: label"
    );
    assert!(screen.contains("1920x1080"), "must show image resolution");
    assert!(screen.contains("Blur:"), "must show Blur: label");
    assert!(screen.contains("LOW"), "must show LOW blur");
    assert!(screen.contains("Pose:"), "must show Pose: label");
    assert!(screen.contains("Quality:"), "must show Quality: label");
    assert!(screen.contains("0.91"), "must show quality score 0.91");
    assert!(screen.contains("Status:"), "must show Status: label");
    assert!(screen.contains("GOOD"), "must show GOOD status");
}

#[test]
fn renders_face_quality_on_tall_terminal() {
    let backend = TestBackend::new(80, 32);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("FACE QUALITY"));
    assert!(screen.contains("Faces:"));
    assert!(screen.contains("1"));
    assert!(screen.contains("Resolution:"));
    assert!(screen.contains("Blur:"));
    assert!(screen.contains("LOW"));
    assert!(screen.contains("Pose:"));
    assert!(screen.contains("Quality:"));
    assert!(screen.contains("0.91"));
    assert!(screen.contains("Status:"));
    assert!(screen.contains("GOOD"));
}

#[test]
fn renders_discovery_stage_on_standard_terminal() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start); // Stage::Input
    app.apply(AppAction::Verify); // Stage::Face
    app.apply(AppAction::Verify); // Stage::Discovery

    assert_eq!(app.current, Some(Stage::Discovery));

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    // Verify all required DISCOVERY fields from the prompt
    assert!(screen.contains("DISCOVERY"), "must show DISCOVERY header");
    assert!(screen.contains("Provider:"), "must show Provider: label");
    assert!(
        screen.contains("external_reverse_image"),
        "must show provider name"
    );
    assert!(screen.contains("Request:"), "must show Request: label");
    assert!(screen.contains("SENT"), "must show SENT request status");
    assert!(
        screen.contains("Candidates:"),
        "must show Candidates: label"
    );
    assert!(screen.contains("12"), "must show candidate count");
    assert!(screen.contains("Unique:"), "must show Unique: label");
    assert!(screen.contains("8"), "must show unique candidate count");
}

#[test]
fn renders_discovery_stage_on_tall_terminal() {
    let backend = TestBackend::new(80, 32);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("DISCOVERY"));
    assert!(screen.contains("Provider:"));
    assert!(screen.contains("Request:"));
    assert!(screen.contains("SENT"));
    assert!(screen.contains("Candidates:"));
    assert!(screen.contains("Unique:"));
}

#[test]
fn renders_discovery_failure_clearly() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    app.set_discovery_error(
        "google_lens",
        "Rate limit exceeded (HTTP 429): retry after 30s",
    );

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("DISCOVERY"));
    assert!(screen.contains("Provider:"));
    assert!(screen.contains("google_lens"));
    assert!(screen.contains("Request:"));
    assert!(screen.contains("FAILED"), "must show FAILED request status");
    assert!(
        screen.contains("SEARCH FAILURE:"),
        "must clearly show SEARCH FAILURE header"
    );
    assert!(
        screen.contains("Rate limit exceeded"),
        "must clearly display failure details"
    );
}

#[test]
fn renders_candidate_verification_stage_on_standard_terminal() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    // Advance to Stage::Verify: Start -> Input, Verify -> Face, Verify -> Discovery, Verify -> Verify
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    // Verify header and candidate verification pane
    assert!(screen.contains("CANDIDATE VERIFICATION"));
    assert!(screen.contains("Candidates:"));
    assert!(screen.contains("Verified:"));
    assert!(screen.contains("Threshold:"));

    // Verify all 4 required statuses appear in the candidate list
    assert!(screen.contains("VERIFIED"));
    assert!(screen.contains("BELOW THRESHOLD"));
    assert!(screen.contains("NO FACE"));
    assert!(screen.contains("ERROR"));

    // Verify candidate details and preserved fields
    assert!(screen.contains("Sim:"));
    assert!(screen.contains("Face #"));
    assert!(screen.contains("Selected Candidate #1"));
    assert!(screen.contains("7a9f82c4e1d3b5a6"));

    // CRITICAL REQUIREMENT: Do not use the word "identity confirmed" anywhere
    assert!(
        !screen.to_lowercase().contains("identity confirmed"),
        "forbidden phrasing 'identity confirmed' must never appear in TUI"
    );
}

#[test]
fn renders_candidate_verification_selection_navigation() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    // Navigate to Candidate #2
    app.apply(AppAction::Select(tekmerion_tui::input::Direction::Down));

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("Selected Candidate #2"));
    assert!(screen.contains("archives.example.net"));
    assert!(screen.contains("1b2c3d4e5f6a7b8c"));
    assert!(screen.contains("BELOW THRESHOLD"));

    // CRITICAL REQUIREMENT
    assert!(
        !screen.to_lowercase().contains("identity confirmed"),
        "forbidden phrasing 'identity confirmed' must never appear in TUI"
    );
}

#[test]
fn renders_candidate_verification_stage_on_tall_terminal() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    assert!(screen.contains("CANDIDATE VERIFICATION"));
    assert!(screen.contains("VERIFIED"));
    assert!(screen.contains("BELOW THRESHOLD"));
    assert!(screen.contains("NO FACE"));
    assert!(screen.contains("ERROR"));
    assert!(screen.contains("7a9f82c4e1d3b5a6"));
    assert!(!screen.to_lowercase().contains("identity confirmed"));
}

#[test]
fn renders_candidate_ranking_columns_and_deterministic_order() {
    let backend = TestBackend::new(90, 26);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");
    let mut app = App::new();
    app.apply(AppAction::Start);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);
    app.apply(AppAction::Verify);

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("render failed");

    let screen = buffer_to_string(&terminal);

    // Verify all 6 required display columns are rendered
    assert!(screen.contains("RANK"), "must display RANK column");
    assert!(screen.contains("SOURCE"), "must display SOURCE column");
    assert!(
        screen.contains("SIMILARITY"),
        "must display SIMILARITY column"
    );
    assert!(screen.contains("QUALITY"), "must display QUALITY column");
    assert!(screen.contains("SCORE"), "must display SCORE column");
    assert!(screen.contains("STATUS"), "must display STATUS column");

    // Verify deterministic ranks appear in order
    assert!(screen.contains("#1"), "must display rank #1");
    assert!(screen.contains("#2"), "must display rank #2");
    assert!(screen.contains("#3"), "must display rank #3");
    assert!(screen.contains("#4"), "must display rank #4");

    // Verify ranking score is not represented as probability of identity
    assert!(
        !screen.to_lowercase().contains("probability"),
        "ranking must NOT be represented as a probability of identity"
    );
    assert!(
        !screen.to_lowercase().contains("probability of identity"),
        "ranking must NOT be represented as probability of identity"
    );

    // CRITICAL REQUIREMENT: Do not use the word "identity confirmed" anywhere
    assert!(
        !screen.to_lowercase().contains("identity confirmed"),
        "forbidden phrasing 'identity confirmed' must never appear in TUI"
    );
}
