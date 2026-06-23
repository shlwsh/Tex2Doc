//! doc-server 库入口（供集成测试复用 router）。

pub mod error;
pub mod limits;
pub mod routes;
pub mod state;
pub mod worker_service;

pub use routes::router as build_router;
