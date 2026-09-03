//! Tests for the TUI module

use hh_face::config::Config;
use hh_face::pipeline::state_machine::PipelineStateMachine;
use hh_face::ui::tui::{init_terminal, restore_terminal};
use std::env;

#[tokio::test]
async fn test_tui_initialization() {
    // Set up test configuration
    let config = Config {
        blockchain_rpc_url: "http://test.com".to_string(),
        contract_address: "0x0".to_string(),
        face_model_path: "./test_model.onnx".to_string(),
        search_api_key: "test_key".to_string(),
    };

    // Initialize state machine
    let state_machine = PipelineStateMachine::new();

    // Test terminal initialization
    let mut terminal = init_terminal().unwrap();

    // Test UI rendering
    terminal
        .draw(|f| hh_face::ui::tui::ui(f, &state_machine, &config))
        .unwrap();

    // Clean up
    restore_terminal().unwrap();
}
