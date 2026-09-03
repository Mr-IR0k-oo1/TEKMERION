//! Search module

pub mod models;
pub mod provider;
pub mod client;
pub mod ranking;

pub use models::{SearchCandidate, SearchProvider};
pub use client::SearchClient;
pub use ranking::rank_candidates;
