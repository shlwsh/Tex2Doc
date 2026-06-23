//! Billing and subscription API methods.

use crate::client::ApiClient;
use crate::models::{
    BillingPortalRequest, BillingSession, CheckoutRequest, PlanSummary, RechargeRecord,
    RedeemCodeOptions, RedeemCodeRecord, RedeemCodeRequest, RedeemCodeResult,
};
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

    pub async fn recharge_records(&self) -> Result<Vec<RechargeRecord>, ApiError> {
        self.get("recharges").await
    }

    pub async fn redeem_code_options(&self) -> Result<RedeemCodeOptions, ApiError> {
        self.get("redeem-codes/options").await
    }

    pub async fn redeem_code(
        &self,
        request: &RedeemCodeRequest,
    ) -> Result<RedeemCodeResult, ApiError> {
        self.post("redeem-codes/redeem", request).await
    }

    pub async fn redeem_code_records(&self) -> Result<Vec<RedeemCodeRecord>, ApiError> {
        self.get("redeem-codes/records").await
    }
}
