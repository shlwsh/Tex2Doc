//! Commercial API client for Tex2Doc cloud services.

pub mod client;
pub mod models;

pub use client::{ApiClient, ClientConfig};
pub use models::{
    ApiError, AnalysisJob, AnalysisResult, CheckResult, DetailedReport, JobStatus, SubmitRequest,
};
