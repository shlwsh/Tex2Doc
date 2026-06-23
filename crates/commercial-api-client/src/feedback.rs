//! Feedback (issue/requirement) API methods.

use crate::client::ApiClient;
use crate::models::{
    AddMessageRequest, CreateFeedbackRequest, CreateFeedbackResponse, FeedbackMessage,
    FeedbackThread, FeedbackThreadDetail,
};
use crate::ApiError;

impl ApiClient {
    /// List feedback threads for the current user.
    pub async fn feedback_threads(&self) -> Result<Vec<FeedbackThread>, ApiError> {
        self.get("feedback/threads").await
    }

    /// Get a single feedback thread with all messages.
    pub async fn feedback_thread(&self, thread_id: &str) -> Result<FeedbackThreadDetail, ApiError> {
        self.get(&format!("feedback/threads/{thread_id}")).await
    }

    /// Create a new feedback thread (with initial message).
    pub async fn create_feedback_thread(
        &self,
        request: &CreateFeedbackRequest,
    ) -> Result<CreateFeedbackResponse, ApiError> {
        self.post("feedback/threads", request).await
    }

    /// Add a message (reply) to an existing feedback thread.
    pub async fn add_feedback_message(
        &self,
        thread_id: &str,
        request: &AddMessageRequest,
    ) -> Result<FeedbackMessage, ApiError> {
        self.post(&format!("feedback/threads/{thread_id}/messages"), request)
            .await
    }
}
