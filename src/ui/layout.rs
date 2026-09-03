//! UI layout module

use crate::app::App;
use crate::pipeline::state::PipelineState;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Application layout
pub struct AppLayout {
    size: Rect,
}

impl AppLayout {
    /// Create a new application layout
    pub fn new(size: Rect) -> Self {
        Self { size }
    }

    /// Render the application UI
    pub fn render(&self, f: &mut Frame, app: &App) {
        let state = app.state.blocking_lock();

        // Header
        let header = Block::default()
            .title("Face Identification & Blockchain Verification")
            .borders(Borders::ALL);
        f.render_widget(header, self.size);

        // Pipeline stages
        let pipeline_stages = self.render_pipeline_stages(&state.pipeline_state);
        f.render_widget(pipeline_stages, self.size.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }));

        // Status
        let status = Paragraph::new(state.status_message.clone())
            .wrap(Wrap { trim: true });
        f.render_widget(status, self.size.inner(&Margin {
            vertical: 3,
            horizontal: 1,
        }));

        // Candidates
        let candidates = self.render_candidates(&state);
        f.render_widget(candidates, self.size.inner(&Margin {
            vertical: 5,
            horizontal: 1,
        }));

        // Evidence
        let evidence = self.render_evidence(&state);
        f.render_widget(evidence, self.size.inner(&Margin {
            vertical: 10,
            horizontal: 1,
        }));

        // Blockchain
        let blockchain = self.render_blockchain(&state);
        f.render_widget(blockchain, self.size.inner(&Margin {
            vertical: 15,
            horizontal: 1,
        }));

        // Footer
        let footer = Block::default()
            .title("Controls: q=quit, r=reset, enter=start, up/down=candidate navigation")
            .borders(Borders::ALL);
        f.render_widget(footer, self.size.inner(&Margin {
            vertical: 17,
            horizontal: 1,
        }));
    }

    /// Render pipeline stages
    fn render_pipeline_stages(&self, current_state: &PipelineState) -> Paragraph {
        let stages = [
            PipelineState::Idle,
            PipelineState::InputReady,
            PipelineState::FaceProcessing { image_path: "".to_string(), face_count: None, embedding_dimensions: None },
            PipelineState::Searching { image_path: "".to_string(), candidate_count: None },
            PipelineState::CandidatesFound { candidates: vec![] },
            PipelineState::Verifying,
            PipelineState::MatchFound,
            PipelineState::EvidenceCreated,
            PipelineState::BlockchainSubmitting,
            PipelineState::BlockchainConfirmed,
            PipelineState::Verified,
            PipelineState::Error,
        ];

        let mut content = String::new();
        for stage in stages {
            let marker = if matches!(current_state, PipelineState::FaceProcessing { .. } if stage == PipelineState::FaceProcessing { .. }) {
                "> "
            } else if matches!(current_state, PipelineState::Searching { .. } if stage == PipelineState::Searching { .. }) {
                "> "
            } else if matches!(current_state, PipelineState::CandidatesFound { .. } if stage == PipelineState::CandidatesFound { .. }) {
                "> "
            } else if &stage == current_state {
                "> "
            } else {
                "  "
            };
            content.push_str(&format!("{}{} \n", marker, stage.display_name()));
        }

        Paragraph::new(content).wrap(Wrap { trim: true })
    }

    /// Render face processing information
    fn render_face_processing(&self, state: &AppState) -> Paragraph {
        if let PipelineState::FaceProcessing { image_path, face_count, embedding_dimensions } = &state.pipeline_state {
            let mut content = String::new();
            content.push_str(&format!("FACE ANALYSIS\n\n"));
            content.push_str(&format!("Input:\n{}\n\n", image_path));

            if let Some(count) = face_count {
                content.push_str(&format!("Faces detected:\n{}\n\n", count));
            } else {
                content.push_str("Faces detected:\n-\n\n");
            }

            if let Some(dimensions) = embedding_dimensions {
                content.push_str(&format!("Embedding:\n{} dimensions\n\n", dimensions));
            } else {
                content.push_str("Embedding:\n-\n\n");
            }

            content.push_str("Status:\nSUCCESS\n");

            Paragraph::new(content).wrap(Wrap { trim: true })
        } else {
            Paragraph::new("").wrap(Wrap { trim: true })
        }
    }

    /// Render search information
    fn render_search(&self, state: &AppState) -> Paragraph {
        if let PipelineState::Searching { image_path, candidate_count } = &state.pipeline_state {
            let mut content = String::new();
            content.push_str(&format!("SEARCH RESULTS\n\n"));
            content.push_str(&format!("Input:\n{}\n\n", image_path));

            if let Some(count) = candidate_count {
                content.push_str(&format!("Found:\n{} candidates\n\n", count));
            } else {
                content.push_str("Found:\n-\n\n");
            }

            Paragraph::new(content).wrap(Wrap { trim: true })
        } else {
            Paragraph::new("").wrap(Wrap { trim: true })
        }
    }

    /// Render candidates
    fn render_candidates(&self, state: &AppState) -> Paragraph {
        if let PipelineState::CandidatesFound { candidates } = &state.pipeline_state {
            let mut content = String::new();
            content.push_str(&format!("SEARCH RESULTS\n\n"));
            content.push_str(&format!("Found:\n{} candidates\n\n", candidates.len()));

            for (i, candidate) in candidates.iter().enumerate() {
                let marker = if i == state.selected_candidate { "> " } else { "  " };
                content.push_str(&format!("{}[{}]\n{}\n\n", marker, i + 1, candidate));
            }

            Paragraph::new(content).wrap(Wrap { trim: true })
        } else {
            Paragraph::new("").wrap(Wrap { trim: true })
        }
    }

    /// Render evidence

    /// Render blockchain information
    fn render_blockchain(&self, state: &AppState) -> Paragraph {
        let content = state.blockchain_tx.clone().unwrap_or_else(|| "No blockchain transaction".to_string());
        Paragraph::new(content).wrap(Wrap { trim: true })
    }
}
