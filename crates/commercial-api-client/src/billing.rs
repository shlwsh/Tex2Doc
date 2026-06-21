//! Billing and subscription API methods.

use crate::client::ApiClient;
use crate::models::{BillingPortalRequest, BillingSession, CheckoutRequest, PlanSummary};
use crate::ApiError;

impl ApiClient {
    pub async fn plans(&self) -> Result<Vec<PlanSummary>, ApiError> {
        self.get("plans").await
    }

    pub async fn create_checkout(
        &self,
        request: &CheckoutRequest,
    ) -> Result<BillingSession, ApiError> {
        self.post("billing/checkout", request).await
    }

    pub async fn create_billing_portal(
        &self,
        request: &BillingPortalRequest,
    ) -> Result<BillingSession, ApiError> {
        self.post("billing/portal", request).await
    }
}
