//! doc-server 库入口（供集成测试复用 router）。

pub mod error;
pub mod limits;
pub mod routes;

pub use routes::router as build_router;
