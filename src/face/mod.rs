//! Face module

pub mod client;
pub mod model;

pub use client::{FaceWorkerClient, FaceWorkerRequest, FaceWorkerResponse, FaceInfo};
pub use model::FaceModel;
