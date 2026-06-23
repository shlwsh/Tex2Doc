//! Release and update API methods.

use crate::client::ApiClient;
use crate::models::ReleaseManifest;
use crate::ApiError;

impl ApiClient {
    pub async fn release_manifest(&self, channel: &str) -> Result<ReleaseManifest, ApiError> {
        self.get(&format!("releases/{channel}")).await
    }
}
