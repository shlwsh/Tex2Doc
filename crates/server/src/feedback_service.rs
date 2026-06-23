//! Feedback (issue / requirement) service with chat-style threaded messages.
//!
//! Data model:
//! - `FeedbackThread`: a top-level feedback submission (linked to a conversion job)
//! - `FeedbackMessage`: individual messages within a thread (user, admin, or system)
//!
//! In this implementation, both structures live in-memory alongside the existing
//! `ServerState` structures. When a PostgreSQL store is wired up, swap the HashMap
//! backends for real DB queries.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::state::now_timestamp;

pub struct FeedbackStore {
    inner: Arc<RwLock<FeedbackStoreInner>>,
}

pub struct FeedbackStoreInner {
    pub threads: HashMap<String, FeedbackThread>,
    pub messages: HashMap<String, Vec<FeedbackMessage>>,
    pub seq: AtomicU64,
}

impl Clone for FeedbackStore {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl std::ops::Deref for FeedbackStore {
    type Target = RwLock<FeedbackStoreInner>;
    fn deref(&self) -> &Self::Target {
        // SAFETY: FeedbackStore always wraps Arc<RwLock<...>>
        // unwrap is safe because we control the constructor
        self.inner.deref()
    }
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FeedbackStoreInner {
                threads: Default::default(),
                messages: Default::default(),
                seq: AtomicU64::new(1),
            })),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Data structures
// ─────────────────────────────────────────────────────────────────────────────

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
        match self {
            FeedbackType::Issue => write!(f, "issue"),
            FeedbackType::Requirement => write!(f, "requirement"),
        }
    }
}

impl std::str::FromStr for FeedbackType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "issue" => Ok(FeedbackType::Issue),
            "requirement" => Ok(FeedbackType::Requirement),
            _ => Err(format!("unknown feedback_type: {s}")),
        }
    }
}

impl FeedbackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackType::Issue => "issue",
            FeedbackType::Requirement => "requirement",
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
        match self {
            FeedbackStatus::Open => write!(f, "open"),
            FeedbackStatus::InProgress => write!(f, "in_progress"),
            FeedbackStatus::Resolved => write!(f, "resolved"),
            FeedbackStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for FeedbackStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(FeedbackStatus::Open),
            "in_progress" => Ok(FeedbackStatus::InProgress),
            "resolved" => Ok(FeedbackStatus::Resolved),
            "closed" => Ok(FeedbackStatus::Closed),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

impl FeedbackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackStatus::Open => "open",
            FeedbackStatus::InProgress => "in_progress",
            FeedbackStatus::Resolved => "resolved",
            FeedbackStatus::Closed => "closed",
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
        match self {
            FeedbackPriority::Low => write!(f, "low"),
            FeedbackPriority::Normal => write!(f, "normal"),
            FeedbackPriority::High => write!(f, "high"),
            FeedbackPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for FeedbackPriority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(FeedbackPriority::Low),
            "normal" => Ok(FeedbackPriority::Normal),
            "high" => Ok(FeedbackPriority::High),
            "urgent" => Ok(FeedbackPriority::Urgent),
            _ => Err(format!("unknown priority: {s}")),
        }
    }
}

impl FeedbackPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeedbackPriority::Low => "low",
            FeedbackPriority::Normal => "normal",
            FeedbackPriority::High => "high",
            FeedbackPriority::Urgent => "urgent",
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

impl std::fmt::Display for SenderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SenderType::User => write!(f, "user"),
            SenderType::Admin => write!(f, "admin"),
            SenderType::System => write!(f, "system"),
        }
    }
}

impl SenderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SenderType::User => "user",
            SenderType::Admin => "admin",
            SenderType::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub url: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Service API
// ─────────────────────────────────────────────────────────────────────────────

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

impl FeedbackStore {
    /// Create a new feedback thread, including the initial user message.
    pub async fn create_thread(
        &self,
        user_id: String,
        req: CreateThreadRequest,
    ) -> Result<(FeedbackThread, FeedbackMessage), FeedbackError> {
        if req.title.trim().is_empty() {
            return Err(FeedbackError::Validation("title is required".into()));
        }
        if req.content.trim().is_empty() {
            return Err(FeedbackError::Validation("content is required".into()));
        }

        let feedback_type: FeedbackType = req
            .feedback_type
            .parse()
            .map_err(|e| FeedbackError::Validation(e))?;
        let priority: FeedbackPriority = req
            .priority
            .as_deref()
            .unwrap_or("normal")
            .parse()
            .map_err(|e| FeedbackError::Validation(e))?;

        let (thread, message) = {
            let mut inner = self.write().await;
            let seq = inner.seq.fetch_add(2, Ordering::Relaxed);
            let thread_id = format!("fbth_{:016x}", seq);
            let message_id = format!("fbtm_{:016x}", seq + 1);
            let now = now_timestamp();

            let thread = FeedbackThread {
                thread_id: thread_id.clone(),
                user_id: user_id.clone(),
                conversion_job_id: req.conversion_job_id,
                title: req.title,
                feedback_type,
                status: FeedbackStatus::Open,
                priority,
                admin_assignee: None,
                message_count: 1,
                latest_message_at: Some(now.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
            };

            let message = FeedbackMessage {
                message_id: message_id.clone(),
                thread_id: thread_id.clone(),
                parent_message_id: None,
                sender_user_id: Some(user_id),
                sender_type: SenderType::User,
                content: req.content,
                attachments: Vec::new(),
                is_internal: false,
                created_at: now,
            };

            inner
                .threads
                .insert(thread_id.clone(), thread.clone());
            inner
                .messages
                .insert(thread_id.clone(), vec![message.clone()]);

            (thread, message)
        };

        Ok((thread, message))
    }

    /// Add a message to an existing thread (user reply).
    pub async fn add_message(
        &self,
        user_id: String,
        thread_id: &str,
        req: AddMessageRequest,
    ) -> Result<FeedbackMessage, FeedbackError> {
        if req.content.trim().is_empty() {
            return Err(FeedbackError::Validation("content is required".into()));
        }

        let message = {
            let mut inner = self.write().await;
            let thread = inner
                .threads
                .get_mut(thread_id)
                .ok_or(FeedbackError::NotFound)?;
            if thread.user_id != user_id {
                return Err(FeedbackError::Forbidden);
            }
            thread.message_count += 1;
            thread.latest_message_at = Some(now_timestamp());
            thread.updated_at = now_timestamp();

            let message_id = format!("fbtm_{:016x}", inner.seq.fetch_add(1, Ordering::Relaxed));
            let message = FeedbackMessage {
                message_id: message_id.clone(),
                thread_id: thread_id.to_string(),
                parent_message_id: req.parent_message_id,
                sender_user_id: Some(user_id),
                sender_type: SenderType::User,
                content: req.content,
                attachments: Vec::new(),
                is_internal: false,
                created_at: now_timestamp(),
            };

            inner
                .messages
                .entry(thread_id.to_string())
                .or_default()
                .push(message.clone());

            message
        };

        Ok(message)
    }

    /// Admin reply to a thread.
    pub async fn admin_reply(
        &self,
        admin_id: String,
        thread_id: &str,
        req: AdminReplyRequest,
    ) -> Result<FeedbackMessage, FeedbackError> {
        if req.content.trim().is_empty() {
            return Err(FeedbackError::Validation("content is required".into()));
        }

        let message = {
            let mut inner = self.write().await;
            let thread = inner
                .threads
                .get_mut(thread_id)
                .ok_or(FeedbackError::NotFound)?;

            // Auto-transition to in_progress when admin first replies
            if thread.status == FeedbackStatus::Open {
                thread.status = FeedbackStatus::InProgress;
            }
            thread.message_count += 1;
            thread.latest_message_at = Some(now_timestamp());
            thread.updated_at = now_timestamp();

            let message_id = format!("fbtm_{:016x}", inner.seq.fetch_add(1, Ordering::Relaxed));
            let message = FeedbackMessage {
                message_id: message_id.clone(),
                thread_id: thread_id.to_string(),
                parent_message_id: None,
                sender_user_id: Some(admin_id),
                sender_type: SenderType::Admin,
                content: req.content,
                attachments: Vec::new(),
                is_internal: req.is_internal.unwrap_or(false),
                created_at: now_timestamp(),
            };

            inner
                .messages
                .entry(thread_id.to_string())
                .or_default()
                .push(message.clone());

            message
        };

        Ok(message)
    }

    /// Admin update thread status/priority/assignee.
    pub async fn admin_update(
        &self,
        thread_id: &str,
        req: AdminUpdateThreadRequest,
    ) -> Result<FeedbackThread, FeedbackError> {
        let mut inner = self.write().await;
        let thread = inner
            .threads
            .get_mut(thread_id)
            .ok_or(FeedbackError::NotFound)?;

        if let Some(s) = req.status {
            thread.status = s.parse().map_err(|e: String| FeedbackError::Validation(e))?;
        }
        if let Some(p) = req.priority {
            thread.priority = p.parse().map_err(|e: String| FeedbackError::Validation(e))?;
        }
        thread.admin_assignee = req.admin_assignee;
        thread.updated_at = now_timestamp();

        Ok(thread.clone())
    }

    /// List threads for a specific user (user-facing).
    pub async fn list_user_threads(&self, user_id: &str) -> Vec<FeedbackThreadSummary> {
        let inner = self.read().await;
        let mut threads: Vec<_> = inner
            .threads
            .values()
            .filter(|t| t.user_id == user_id)
            .map(|t| FeedbackThreadSummary {
                thread_id: t.thread_id.clone(),
                conversion_job_id: t.conversion_job_id.clone(),
                title: t.title.clone(),
                feedback_type: t.feedback_type.as_str().to_string(),
                status: t.status.as_str().to_string(),
                priority: t.priority.as_str().to_string(),
                message_count: t.message_count,
                latest_message_at: t.latest_message_at.clone(),
                created_at: t.created_at.clone(),
                updated_at: t.updated_at.clone(),
            })
            .collect();
        threads.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        threads
    }

    /// List all threads with optional filters (admin-facing).
    pub async fn admin_list(&self, filters: &ThreadFilters) -> Vec<FeedbackThreadSummary> {
        let page = filters.page.unwrap_or(1).max(1);
        let page_size = filters.page_size.unwrap_or(20).min(100);

        let inner = self.read().await;
        let mut threads: Vec<_> = inner.threads.values().filter_map(|t| {
            if let Some(ref s) = filters.status {
                if t.status.as_str() != s.as_str() {
                    return None;
                }
            }
            if let Some(ref ft) = filters.feedback_type {
                if t.feedback_type.as_str() != ft.as_str() {
                    return None;
                }
            }
            if let Some(ref from) = filters.date_from {
                if &t.created_at < from {
                    return None;
                }
            }
            if let Some(ref to) = filters.date_to {
                if &t.created_at > to {
                    return None;
                }
            }

            Some(FeedbackThreadSummary {
                thread_id: t.thread_id.clone(),
                conversion_job_id: t.conversion_job_id.clone(),
                title: t.title.clone(),
                feedback_type: t.feedback_type.as_str().to_string(),
                status: t.status.as_str().to_string(),
                priority: t.priority.as_str().to_string(),
                message_count: t.message_count,
                latest_message_at: t.latest_message_at.clone(),
                created_at: t.created_at.clone(),
                updated_at: t.updated_at.clone(),
            })
        }).collect();

        threads.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let start = ((page - 1) * page_size) as usize;
        threads.into_iter().skip(start).take(page_size as usize).collect()
    }

    /// Get a single thread with all messages (user must own the thread).
    pub async fn get_thread_for_user(
        &self,
        user_id: &str,
        thread_id: &str,
    ) -> Result<(FeedbackThread, Vec<FeedbackMessage>), FeedbackError> {
        let inner = self.read().await;
        let thread = inner
            .threads
            .get(thread_id)
            .ok_or(FeedbackError::NotFound)?;
        if thread.user_id != user_id {
            return Err(FeedbackError::Forbidden);
        }
        let messages = inner
            .messages
            .get(thread_id)
            .cloned()
            .unwrap_or_default();
        Ok((thread.clone(), messages))
    }

    /// Get a single thread with all messages (admin — any thread).
    #[allow(dead_code)]
    pub async fn admin_get_thread(
        &self,
        thread_id: &str,
    ) -> Result<(FeedbackThread, Vec<FeedbackMessage>), FeedbackError> {
        let inner = self.read().await;
        let thread = inner
            .threads
            .get(thread_id)
            .ok_or(FeedbackError::NotFound)?;
        let messages = inner
            .messages
            .get(thread_id)
            .cloned()
            .unwrap_or_default();
        Ok((thread.clone(), messages))
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
    #[allow(dead_code)]
    pub enum FeedbackError {
        #[error("validation error: {0}")]
        Validation(String),
        #[error("thread not found")]
        NotFound,
        #[error("forbidden")]
        Forbidden,
        #[error("forbidden")]
        Unauthorized,
    }
