//! doc-server 库入口（供集成测试复用 router）。

pub mod db_store;
pub mod error;
pub mod error_code;
pub mod excel_export;
pub mod feedback_service;
pub mod file_storage;
pub mod limits;
pub mod routes;
pub mod state;
pub mod worker_service;

pub use routes::router as build_router;
