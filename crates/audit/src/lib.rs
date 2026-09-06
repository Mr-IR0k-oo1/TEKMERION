//! TEKMERION audit trail, provenance tracking, and persistent run bundle management.

pub mod error;
pub mod events;
pub mod logger;
pub mod persistence;

pub use error::AuditError;
pub use events::{AuditEvent, EventType};
pub use logger::AuditLogger;
pub use persistence::{RunBundleManager, RunBundleMeta};
