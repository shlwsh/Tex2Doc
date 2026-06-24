//! Feedback service backed by PostgreSQL.

use serde::{Deserialize, Serialize};

use crate::db_store::DbStore;

#[derive(Clone)]
pub struct FeedbackStore {
    db: DbStore,
}

impl FeedbackStore {
    pub fn new(db: DbStore) -> Self {
        Self { db }
    }

    pub async fn create_thread(
        &self,
        user_id: String,
        req: CreateThreadRequest,
    ) -> Result<(FeedbackThread, FeedbackMessage), FeedbackError> {
        self.db.create_feedback_thread(user_id, req).await
    }

    pub async fn add_message(
        &self,
        user_id: String,
        thread_id: &str,
        req: AddMessageRequest,
    ) -> Result<FeedbackMessage, FeedbackError> {
        self.db.add_feedback_message(user_id, thread_id, req).await
    }

    pub async fn admin_reply(
        &self,
        admin_id: String,
        thread_id: &str,
        req: AdminReplyRequest,
    ) -> Result<FeedbackMessage, FeedbackError> {
        self.db
            .admin_reply_feedback_message(admin_id, thread_id, req)
            .await
    }

    pub async fn admin_update(
        &self,
        thread_id: &str,
        req: AdminUpdateThreadRequest,
    ) -> Result<FeedbackThread, FeedbackError> {
        self.db.admin_update_feedback_thread(thread_id, req).await
    }

    pub async fn list_user_threads(&self, user_id: &str) -> Vec<FeedbackThreadSummary> {
        self.db
            .list_user_feedback_threads(user_id)
            .await
            .unwrap_or_default()
    }

    pub async fn admin_list(&self, filters: &ThreadFilters) -> Vec<FeedbackThreadSummary> {
        self.db
            .admin_list_feedback_threads(filters)
            .await
            .unwrap_or_default()
    }

    pub async fn get_thread_for_user(
        &self,
        user_id: &str,
        thread_id: &str,
    ) -> Result<(FeedbackThread, Vec<FeedbackMessage>), FeedbackError> {
        self.db.get_feedback_thread_for_user(user_id, thread_id).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackThread {
    pub thread_id: String,
    pub user_id: String,
    pub conversion_job_id: Option<String>,
    pub title: String,
    pub feedback_type: FeedbackType,
    pub status: FeedbackStatus,
    pub priority: FeedbackPriority,
    pub admin_assignee: Option<String>,
    pub message_count: u32,
    pub latest_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackType {
    Issue,
    Requirement,
}

impl std::fmt::Display for FeedbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FeedbackType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "issue" => Ok(Self::Issue),
            "requirement" => Ok(Self::Requirement),
            _ => Err(format!("unknown feedback_type: {s}")),
        }
    }
}

impl FeedbackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::Requirement => "requirement",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
}

impl std::fmt::Display for FeedbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FeedbackStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "resolved" => Ok(Self::Resolved),
            "closed" => Ok(Self::Closed),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

impl FeedbackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for FeedbackPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for FeedbackPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(format!("unknown priority: {s}")),
        }
    }
}

impl FeedbackPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackMessage {
    pub message_id: String,
    pub thread_id: String,
    pub parent_message_id: Option<String>,
    pub sender_user_id: Option<String>,
    pub sender_type: SenderType,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub is_internal: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SenderType {
    User,
    Admin,
    System,
}

impl SenderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub conversion_job_id: Option<String>,
    pub title: String,
    pub feedback_type: String,
    pub content: String,
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub content: String,
    pub parent_message_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminReplyRequest {
    pub content: String,
    pub is_internal: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateThreadRequest {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub admin_assignee: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ThreadFilters {
    pub status: Option<String>,
    pub feedback_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl Default for ThreadFilters {
    fn default() -> Self {
        Self {
            status: None,
            feedback_type: None,
            date_from: None,
            date_to: None,
            page: Some(1),
            page_size: Some(20),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackThreadSummary {
    pub thread_id: String,
    pub conversion_job_id: Option<String>,
    pub title: String,
    pub feedback_type: String,
    pub status: String,
    pub priority: String,
    pub message_count: u32,
    pub latest_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FeedbackError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("thread not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("unauthorized")]
    Unauthorized,
}
