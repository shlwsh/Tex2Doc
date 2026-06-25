use std::sync::Arc;

use crate::app_state::AppState;
use crate::ui::MainWindow;

pub mod account;
pub mod billing;
pub mod conversion;
pub mod diagnostics;
pub mod helpers;
pub mod history;
pub mod settings;
pub mod update;

/// Wire all UI callbacks to the Rust side.
///
/// Order is intentional: update → account → billing → settings → conversion → history → diagnostics.
/// This matches the push model used in the pre-refactor `main.rs` to avoid callback re-entry issues.
pub fn wire_all(ui: &MainWindow, state: Arc<AppState>) {
    update::wire_update(ui, Arc::clone(&state));
    account::wire_account(ui, Arc::clone(&state));
    billing::wire_billing(ui, Arc::clone(&state));
    billing::wire_billing_cloud(ui, Arc::clone(&state));
    settings::wire_settings(ui, Arc::clone(&state));
    conversion::wire_conversion(ui, Arc::clone(&state));
    history::wire_history(ui, Arc::clone(&state));
    diagnostics::wire_diagnostics(ui, Arc::clone(&state));
}
