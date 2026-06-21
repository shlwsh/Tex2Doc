//! Usage and quota API methods.

use crate::client::ApiClient;
use crate::models::UsageSummary;
use crate::ApiError;

impl ApiClient {
    pub async fn usage(&self) -> Result<UsageSummary, ApiError> {
        self.get("usage").await
    }
}
