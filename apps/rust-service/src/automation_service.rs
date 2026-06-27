//! Automation R&D Service
//! Handles AI-powered automated development workflow

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::result::Result as StdResult;
use axum::http::StatusCode;
use uuid::Uuid;

use crate::error::ApiError;

// ─────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRequest {
    pub id: String,
    pub short_id: String,
    pub source_type: String,
    pub source_id: String,
    pub feedback_thread_id: Option<String>,
    pub conversion_job_id: Option<String>,
    pub title: String,
    pub request_type: String,
    pub status: String,
    pub priority: String,
    pub risk_level: String,
    pub ai_summary: Option<String>,
    pub impact_check_result: Value,
    pub acceptance_criteria: Value,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<String>,
    pub branch_name: Option<String>,
    pub worktree_path: Option<String>,
    pub pr_url: Option<String>,
    pub ci_run_url: Option<String>,
    pub local_validation_log: Option<String>,
    pub deployed_version: Option<String>,
    pub admin_approver_id: Option<String>,
    pub approved_at: Option<String>,
    pub admin_rejector_id: Option<String>,
    pub rejected_at: Option<String>,
    pub rejection_reason: Option<String>,
    pub escalated_to: Option<String>,
    pub escalated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationEvent {
    pub id: String,
    pub request_id: String,
    pub event_type: String,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub message: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationAgent {
    pub id: String,
    pub hostname: String,
    pub agent_version: String,
    pub status: String,
    pub current_request_id: Option<String>,
    pub capabilities: Value,
    pub total_tasks_completed: i32,
    pub total_tasks_failed: i32,
    pub last_heartbeat_at: String,
    pub last_task_at: Option<String>,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSummary {
    pub pending_approval: i64,
    pub waiting_dev: i64,
    pub in_development: i64,
    pub local_failed: i64,
    pub ci_failed: i64,
    pub deployed: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestFilters {
    pub status: Option<String>,
    pub risk_level: Option<String>,
    pub source_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionRequest {
    pub reason: Option<String>,
    pub operator_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────
// Helper functions for row parsing
// ─────────────────────────────────────────────────────────────

fn automation_request_from_row(row: &sqlx::postgres::PgRow) -> AutomationRequest {
    AutomationRequest {
        id: row.get("id"),
        short_id: row.get("short_id"),
        source_type: row.get("source_type"),
        source_id: row.get("source_id"),
        feedback_thread_id: row.get("feedback_thread_id"),
        conversion_job_id: row.get("conversion_job_id"),
        title: row.get("title"),
        request_type: row.get("request_type"),
        status: row.get("status"),
        priority: row.get("priority"),
        risk_level: row.get("risk_level"),
        ai_summary: row.get("ai_summary"),
        impact_check_result: row.get("impact_check_result"),
        acceptance_criteria: row.get("acceptance_criteria"),
        claimed_by: row.get("claimed_by"),
        claimed_at: row.get("claimed_at"),
        branch_name: row.get("branch_name"),
        worktree_path: row.get("worktree_path"),
        pr_url: row.get("pr_url"),
        ci_run_url: row.get("ci_run_url"),
        local_validation_log: row.get("local_validation_log"),
        deployed_version: row.get("deployed_version"),
        admin_approver_id: row.get("admin_approver_id"),
        approved_at: row.get("approved_at"),
        admin_rejector_id: row.get("admin_rejector_id"),
        rejected_at: row.get("rejected_at"),
        rejection_reason: row.get("rejection_reason"),
        escalated_to: row.get("escalated_to"),
        escalated_at: row.get("escalated_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn automation_event_from_row(row: &sqlx::postgres::PgRow) -> AutomationEvent {
    AutomationEvent {
        id: row.get("id"),
        request_id: row.get("request_id"),
        event_type: row.get("event_type"),
        actor_type: row.get("actor_type"),
        actor_id: row.get("actor_id"),
        actor_name: row.get("actor_name"),
        from_status: row.get("from_status"),
        to_status: row.get("to_status"),
        message: row.get("message"),
        payload: row.get("payload"),
        created_at: row.get("created_at"),
    }
}

fn automation_agent_from_row(row: &sqlx::postgres::PgRow) -> AutomationAgent {
    AutomationAgent {
        id: row.get("id"),
        hostname: row.get("hostname"),
        agent_version: row.get("agent_version"),
        status: row.get("status"),
        current_request_id: row.get("current_request_id"),
        capabilities: row.get("capabilities"),
        total_tasks_completed: row.get("total_tasks_completed"),
        total_tasks_failed: row.get("total_tasks_failed"),
        last_heartbeat_at: row.get("last_heartbeat_at"),
        last_task_at: row.get("last_task_at"),
        registered_at: row.get("registered_at"),
    }
}

// ─────────────────────────────────────────────────────────────
// Automation Service
// ─────────────────────────────────────────────────────────────

pub struct AutomationService {
    pub pool: PgPool,
}

impl AutomationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Summary
    pub async fn get_summary(&self) -> StdResult<AutomationSummary, ApiError> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status IN ('triaged', 'needs_approval')) as pending_approval,
                COUNT(*) FILTER (WHERE status = 'queued_for_dev') as waiting_dev,
                COUNT(*) FILTER (WHERE status IN ('claimed', 'coding', 'local_validating')) as in_development,
                COUNT(*) FILTER (WHERE status = 'local_failed') as local_failed,
                COUNT(*) FILTER (WHERE status = 'ci_failed') as ci_failed,
                COUNT(*) FILTER (WHERE status IN ('production_deployed', 'notified')) as deployed,
                COUNT(*) as total
            FROM automation_requests
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        Ok(AutomationSummary {
            pending_approval: row.get("pending_approval"),
            waiting_dev: row.get("waiting_dev"),
            in_development: row.get("in_development"),
            local_failed: row.get("local_failed"),
            ci_failed: row.get("ci_failed"),
            deployed: row.get("deployed"),
            total: row.get("total"),
        })
    }

    // List requests with filters
    pub async fn list_requests(&self, filters: &RequestFilters) -> StdResult<Vec<AutomationRequest>, ApiError> {
        let limit = filters.limit.unwrap_or(50).min(200);
        let offset = filters.offset.unwrap_or(0);

        let mut query = String::from(
            r#"
            SELECT * FROM automation_requests WHERE 1=1
            "#,
        );

        if let Some(ref status) = filters.status {
            if status != "all" {
                query.push_str(&format!(" AND status = '{}'", status));
            }
        }

        if let Some(ref risk) = filters.risk_level {
            if risk != "all" {
                query.push_str(&format!(" AND risk_level = '{}'", risk));
            }
        }

        if let Some(ref source) = filters.source_type {
            if source != "all" {
                query.push_str(&format!(" AND source_type = '{}'", source));
            }
        }

        if let Some(ref search) = filters.search {
            if !search.is_empty() {
                query.push_str(&format!(
                    " AND (title ILIKE '%{}%' OR short_id ILIKE '%{}%' OR source_id ILIKE '%{}%')",
                    search, search, search
                ));
            }
        }

        query.push_str(&format!(
            " ORDER BY created_at DESC LIMIT {} OFFSET {}",
            limit, offset
        ));

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        let requests = rows.iter().map(automation_request_from_row).collect();
        Ok(requests)
    }

    // Get single request
    pub async fn get_request(&self, id: &str) -> StdResult<Option<AutomationRequest>, ApiError> {
        // Try UUID first, then short_id
        let rows = if let Ok(uuid) = Uuid::parse_str(id) {
            sqlx::query("SELECT * FROM automation_requests WHERE id = $1")
                .bind(uuid)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?
        } else {
            sqlx::query("SELECT * FROM automation_requests WHERE short_id = $1")
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?
        };

        Ok(rows.first().map(automation_request_from_row))
    }

    // Get events for a request
    pub async fn get_events(&self, request_id: &str) -> StdResult<Vec<AutomationEvent>, ApiError> {
        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        let rows = sqlx::query(
            r#"
            SELECT * FROM automation_request_events
            WHERE request_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        let events = rows.iter().map(automation_event_from_row).collect();
        Ok(events)
    }

    // Approve request
    pub async fn approve(&self, request_id: &str, operator_id: &str) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        // Check if already approved
        if request.status == "queued_for_dev" || request.admin_approver_id.is_some() {
            return Err(ApiError::BadRequest { code: "already_approved", message: "Request already approved".to_string() });
        }

        // Check risk level - high/critical cannot be auto-approved
        if request.risk_level == "high" || request.risk_level == "critical" {
            return Err(ApiError::BadRequest { code: "auto_approve_denied", message: "High/Critical risk requests cannot be auto-approved".to_string() });
        }

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;
        let operator_uuid = Uuid::parse_str(operator_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid operator id".to_string() })?;

        // Update status
        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = 'queued_for_dev',
                admin_approver_id = $1,
                approved_at = now()
            WHERE id = $2
            "#,
        )
        .bind(operator_uuid)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        // Record event
        self.record_event(
            request_id,
            "approved",
            "admin",
            Some(operator_id),
            None,
            Some(&request.status),
            Some("queued_for_dev"),
            "Request approved by admin",
            json!({}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // Reject request
    pub async fn reject(
        &self,
        request_id: &str,
        operator_id: &str,
        reason: &str,
    ) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        if request.status == "rejected" || request.status == "closed" {
            return Err(ApiError::BadRequest { code: "already_closed", message: "Request already closed".to_string() });
        }

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;
        let operator_uuid = Uuid::parse_str(operator_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid operator id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = 'rejected',
                admin_rejector_id = $1,
                rejected_at = now(),
                rejection_reason = $2
            WHERE id = $3
            "#,
        )
        .bind(operator_uuid)
        .bind(reason)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            "rejected",
            "admin",
            Some(operator_id),
            None,
            Some(&request.status),
            Some("rejected"),
            &format!("Request rejected: {}", reason),
            json!({"reason": reason}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // Escalate to human
    pub async fn escalate(
        &self,
        request_id: &str,
        operator_id: &str,
        assignee: &str,
    ) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = 'needs_human',
                escalated_to = $1,
                escalated_at = now()
            WHERE id = $2
            "#,
        )
        .bind(assignee)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            "escalated",
            "admin",
            Some(operator_id),
            None,
            Some(&request.status),
            Some("needs_human"),
            &format!("Request escalated to human: {}", assignee),
            json!({"assignee": assignee}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // Retry failed request
    pub async fn retry(&self, request_id: &str, operator_id: &str) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        let valid_retry_statuses = ["local_failed", "ci_failed", "blocked"];
        if !valid_retry_statuses.contains(&request.status.as_str()) {
            return Err(ApiError::BadRequest { code: "cannot_retry", message: format!("Cannot retry request in status: {}", request.status) });
        }

        // Determine next status based on where it failed
        let next_status = match request.status.as_str() {
            "local_failed" => "coding",
            "ci_failed" => "pr_open",
            _ => "queued_for_dev",
        };

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = $1
            WHERE id = $2
            "#,
        )
        .bind(next_status)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            "retry_triggered",
            "admin",
            Some(operator_id),
            None,
            Some(&request.status),
            Some(next_status),
            &format!("Retry triggered from {}", request.status),
            json!({"retry_from": request.status}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // List agents
    pub async fn list_agents(&self) -> StdResult<Vec<AutomationAgent>, ApiError> {
        let rows = sqlx::query("SELECT * FROM automation_agents ORDER BY last_heartbeat_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        let agents = rows.iter().map(automation_agent_from_row).collect();
        Ok(agents)
    }

    // Pause agent
    pub async fn pause_agent(&self, agent_id: &str) -> StdResult<AutomationAgent, ApiError> {
        let agent = self.get_agent(agent_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Agent {} not found", agent_id))
        })?;

        if agent.status == "paused" {
            return Err(ApiError::BadRequest { code: "already_paused", message: "Agent already paused".to_string() });
        }

        sqlx::query("UPDATE automation_agents SET status = 'paused' WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.get_agent(agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found after update".to_string()))
    }

    // Resume agent
    pub async fn resume_agent(&self, agent_id: &str) -> StdResult<AutomationAgent, ApiError> {
        let agent = self.get_agent(agent_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Agent {} not found", agent_id))
        })?;

        if agent.status != "paused" {
            return Err(ApiError::BadRequest { code: "not_paused", message: "Agent is not paused".to_string() });
        }

        sqlx::query("UPDATE automation_agents SET status = 'online' WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.get_agent(agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found after update".to_string()))
    }

    // Register agent
    pub async fn register_agent(
        &self,
        agent_id: &str,
        hostname: &str,
        version: &str,
        capabilities: Value,
    ) -> StdResult<AutomationAgent, ApiError> {
        // Upsert agent
        sqlx::query(
            r#"
            INSERT INTO automation_agents (id, hostname, agent_version, capabilities)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                hostname = EXCLUDED.hostname,
                agent_version = EXCLUDED.agent_version,
                capabilities = EXCLUDED.capabilities,
                last_heartbeat_at = now(),
                status = 'online'
            "#,
        )
        .bind(agent_id)
        .bind(hostname)
        .bind(version)
        .bind(capabilities)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.get_agent(agent_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Agent not found after registration".to_string()))
    }

    // Agent heartbeat
    pub async fn agent_heartbeat(&self, agent_id: &str) -> StdResult<(), ApiError> {
        sqlx::query(
            "UPDATE automation_agents SET last_heartbeat_at = now() WHERE id = $1",
        )
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        Ok(())
    }

    // Claim request
    pub async fn claim_request(&self, agent_id: &str, request_id: &str) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        if request.status != "queued_for_dev" {
            return Err(ApiError::BadRequest { code: "not_claimable", message: format!("Request is not available for claiming (status: {})", request.status) });
        }

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = 'claimed',
                claimed_by = $1,
                claimed_at = now()
            WHERE id = $2
            "#,
        )
        .bind(agent_id)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        sqlx::query(
            "UPDATE automation_agents SET status = 'busy', current_request_id = $1, last_task_at = now() WHERE id = $2",
        )
        .bind(uuid)
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            "agent_claimed",
            "agent",
            Some(agent_id),
            None,
            Some(&request.status),
            Some("claimed"),
            &format!("Request claimed by agent {}", agent_id),
            json!({"agent_id": agent_id}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after claim".to_string()))
    }

    // Update local validation result
    pub async fn update_local_validation(
        &self,
        request_id: &str,
        agent_id: &str,
        passed: bool,
        log: &str,
    ) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        let (new_status, event_type) = if passed {
            ("pr_open", "local_validation_passed")
        } else {
            ("local_failed", "local_validation_failed")
        };

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = $1,
                local_validation_log = $2
            WHERE id = $3
            "#,
        )
        .bind(new_status)
        .bind(log)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            event_type,
            "agent",
            Some(agent_id),
            None,
            Some(&request.status),
            Some(new_status),
            if passed { "Local validation passed" } else { "Local validation failed" },
            json!({"passed": passed, "log_length": log.len()}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // Update PR info
    pub async fn update_pr(&self, request_id: &str, agent_id: &str, pr_url: &str) -> StdResult<AutomationRequest, ApiError> {
        let request = self.get_request(request_id).await?.ok_or_else(|| {
            ApiError::NotFound(format!("Request {} not found", request_id))
        })?;

        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            UPDATE automation_requests
            SET status = 'ci_running',
                pr_url = $1
            WHERE id = $2
            "#,
        )
        .bind(pr_url)
        .bind(uuid)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        self.record_event(
            request_id,
            "pr_opened",
            "agent",
            Some(agent_id),
            None,
            Some(&request.status),
            Some("ci_running"),
            &format!("PR opened: {}", pr_url),
            json!({"pr_url": pr_url}),
        )
        .await?;

        self.get_request(request_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after update".to_string()))
    }

    // Helper: get agent
    async fn get_agent(&self, agent_id: &str) -> StdResult<Option<AutomationAgent>, ApiError> {
        let rows = sqlx::query("SELECT * FROM automation_agents WHERE id = $1")
            .bind(agent_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        Ok(rows.first().map(automation_agent_from_row))
    }

    // Helper: record event
    async fn record_event(
        &self,
        request_id: &str,
        event_type: &str,
        actor_type: &str,
        actor_id: Option<&str>,
        actor_name: Option<&str>,
        from_status: Option<&str>,
        to_status: Option<&str>,
        message: &str,
        payload: Value,
    ) -> StdResult<(), ApiError> {
        let uuid = Uuid::parse_str(request_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid request id".to_string() })?;

        sqlx::query(
            r#"
            INSERT INTO automation_request_events
            (request_id, event_type, actor_type, actor_id, actor_name, from_status, to_status, message, payload)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(uuid)
        .bind(event_type)
        .bind(actor_type)
        .bind(actor_id)
        .bind(actor_name)
        .bind(from_status)
        .bind(to_status)
        .bind(message)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        Ok(())
    }

    // Create request from feedback
    pub async fn create_from_feedback(
        &self,
        feedback_id: &str,
        title: &str,
        feedback_type: &str,
        priority: &str,
    ) -> StdResult<AutomationRequest, ApiError> {
        let short_id: String = sqlx::query_scalar(
            "SELECT generate_automation_short_id()"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        let request_type = match feedback_type {
            "issue" => "bug",
            _ => "requirement",
        };

        let risk_level = match priority {
            "urgent" => "high",
            "high" => "medium",
            _ => "low",
        };

        let id = Uuid::new_v4();
        let feedback_uuid = Uuid::parse_str(feedback_id)
            .map_err(|_| ApiError::BadRequest { code: "invalid_id", message: "invalid feedback id".to_string() })?;

        sqlx::query(
            r#"
            INSERT INTO automation_requests
            (id, short_id, source_type, source_id, feedback_thread_id, title, request_type, status, priority, risk_level, ai_summary)
            VALUES ($1, $2, 'feedback', $3, $3, $4, $5, 'submitted', $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(&short_id)
        .bind(feedback_uuid)
        .bind(title)
        .bind(request_type)
        .bind(priority)
        .bind(risk_level)
        .bind(&format!("Auto-generated from feedback: {}", title))
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Coded { status: StatusCode::INTERNAL_SERVER_ERROR, code: "db_error", message: e.to_string() })?;

        // Record creation event
        self.record_event(
            &id.to_string(),
            "request_created",
            "system",
            None,
            None,
            None,
            Some("submitted"),
            "Automation request created from feedback",
            json!({"feedback_id": feedback_id}),
        )
        .await?;

        self.get_request(&id.to_string())
            .await?
            .ok_or_else(|| ApiError::NotFound("Request not found after creation".to_string()))
    }
}
