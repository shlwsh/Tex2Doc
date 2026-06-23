//! Commercial API client for Tex2Doc cloud services.

pub mod auth;
pub mod billing;
pub mod client;
pub mod conversions;
pub mod models;
pub mod releases;
pub mod uploads;
pub mod usage;

pub use client::{ApiClient, ClientConfig};
pub use models::{
    AnalysisJob, AnalysisResult, ApiError, AuthResponse, BillingPortalRequest, BillingSession,
    CheckResult, CheckoutRequest, ConversionJob, ConversionReport, CreateConversionRequest,
    DetailedReport, JobStatus, LoginRequest, PlanSummary, RechargeRecord, RedeemCodeOptions,
    RedeemCodeRecord, RedeemCodeRequest, RedeemCodeResult, RedeemPackageSummary, RefreshRequest,
    RegisterRequest, ReleaseManifest, SubmitRequest, UploadResponse, UsageSummary, UserProfile,
};
