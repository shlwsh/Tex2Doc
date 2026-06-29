//! 日志记录基础设施。
//!
//! 提供：
//! - JSON/compact 双格式日志输出（stdout + 文件）
//! - 按大小滚动的文件 writer
//! - TraceID 生成与透传
//! - 敏感字段脱敏

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::HttpBody,
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    response::Response,
};
use chrono::Local;
use sha2::{Digest, Sha256};
use tower::{Layer, Service};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, prelude::*};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// 配置
// ─────────────────────────────────────────────────────────────────────────────

/// 日志配置
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志目录
    pub log_dir: PathBuf,
    /// 日志格式：json 或 compact
    pub format: LogFormat,
    /// 是否输出到 stdout
    pub log_to_stdout: bool,
    /// 单文件最大大小（MB）
    pub max_file_size_mb: u64,
    /// 保留文件数
    pub max_files: usize,
    /// TraceID header 名称
    pub trace_header: String,
    /// 是否脱敏 PII
    pub redact_pii: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs/rust-service"),
            format: LogFormat::Json,
            log_to_stdout: true,
            max_file_size_mb: 128,
            max_files: 30,
            trace_header: "X-Trace-Id".to_string(),
            redact_pii: true,
        }
    }
}

impl LogConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            log_dir: std::env::var("TEX2DOC_LOG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("logs/rust-service")),
            format: std::env::var("TEX2DOC_LOG_FORMAT")
                .map(|v| match v.as_str() {
                    "compact" => LogFormat::Compact,
                    _ => LogFormat::Json,
                })
                .unwrap_or(LogFormat::Json),
            log_to_stdout: std::env::var("TEX2DOC_LOG_TO_STDOUT")
                .map(|v| v != "false")
                .unwrap_or(true),
            max_file_size_mb: std::env::var("TEX2DOC_LOG_MAX_FILE_SIZE_MB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(128),
            max_files: std::env::var("TEX2DOC_LOG_MAX_FILES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            trace_header: std::env::var("TEX2DOC_TRACE_HEADER")
                .unwrap_or_else(|_| "X-Trace-Id".to_string()),
            redact_pii: std::env::var("TEX2DOC_LOG_REDACT_PII")
                .map(|v| v != "false")
                .unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Json,
    Compact,
}

/// 日志文件类型
#[derive(Debug, Clone, Copy)]
pub enum LogFile {
    Service,
    Api,
    Db,
    Security,
}

impl LogFile {
    pub fn name(&self) -> &'static str {
        match self {
            LogFile::Service => "service",
            LogFile::Api => "api",
            LogFile::Db => "db",
            LogFile::Security => "security",
        }
    }

    pub fn ext(&self) -> &'static str {
        "log"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 全局状态
// ─────────────────────────────────────────────────────────────────────────────

static LOG_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

/// 初始化日志系统
pub fn init() {
    let config = LogConfig::from_env();

    // 确保日志目录存在
    if let Err(e) = fs::create_dir_all(&config.log_dir) {
        eprintln!(
            "FATAL: failed to create log directory {:?}: {e}",
            config.log_dir
        );
        std::process::exit(1);
    }

    // 构建 EnvFilter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 创建文件 appender
    let file = SizeRotatingFile::new(
        config.log_dir.join("service.log"),
        config.max_file_size_mb * 1024 * 1024,
        config.max_files,
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    // 存储 guard 防止被 drop
    if let Ok(mut guard_lock) = LOG_GUARD.lock() {
        *guard_lock = Some(guard);
    }

    // 根据配置初始化
    match config.format {
        LogFormat::Json => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true);

            if config.log_to_stdout {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true);

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(stdout_layer)
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            }
        }
        LogFormat::Compact => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .compact()
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(true)
                .with_line_number(true);

            if config.log_to_stdout {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .compact()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true);

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .with(stdout_layer)
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            }
        }
    }

    tracing::info!(
        log_dir = %config.log_dir.display(),
        format = ?config.format,
        log_to_stdout = config.log_to_stdout,
        max_file_size_mb = config.max_file_size_mb,
        max_files = config.max_files,
        "logging initialized"
    );
}

/// 构建文件层
fn build_file_layer(
    config: &LogConfig,
) -> (
    Option<WorkerGuard>,
    Box<dyn tracing_subscriber::layer::Layer<tracing_subscriber::Registry> + Send + Sync>,
) {
    let file = SizeRotatingFile::new(
        config.log_dir.join("service.log"),
        config.max_file_size_mb * 1024 * 1024,
        config.max_files,
    );
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let layer: Box<dyn tracing_subscriber::layer::Layer<tracing_subscriber::Registry> + Send + Sync> =
        match config.format {
            LogFormat::Json => Box::new(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true),
            ),
            LogFormat::Compact => Box::new(
                tracing_subscriber::fmt::layer()
                    .compact()
                    .with_writer(non_blocking)
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(true)
                    .with_line_number(true),
            ),
        };

    (Some(guard), layer)
}

// ─────────────────────────────────────────────────────────────────────────────
// 大小滚动文件 Writer
// ─────────────────────────────────────────────────────────────────────────────

/// 按大小滚动的文件 Writer
pub struct SizeRotatingFile {
    path: PathBuf,
    max_size: u64,
    max_files: usize,
    current_file: Mutex<Option<BufWriter<File>>>,
    current_size: Mutex<u64>,
}

impl SizeRotatingFile {
    /// 创建新的滚动文件
    pub fn new(path: PathBuf, max_size: u64, max_files: usize) -> Self {
        Self {
            path,
            max_size,
            max_files,
            current_file: Mutex::new(None),
            current_size: Mutex::new(0),
        }
    }

    fn get_or_create_file(&self) -> io::Result<BufWriter<File>> {
        let mut file_lock = self.current_file.lock().unwrap();
        let mut size_lock = self.current_size.lock().unwrap();

        // 检查是否需要滚动
        if *size_lock >= self.max_size {
            drop(file_lock);
            self.rotate()?;
            file_lock = self.current_file.lock().unwrap();
        }

        // 如果文件未打开，创建它
        if file_lock.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            let size = file.metadata()?.len();
            *file_lock = Some(BufWriter::new(file));
            *size_lock = size;
        }

        Ok(file_lock.take().unwrap())
    }

    fn rotate(&self) -> io::Result<()> {
        // 关闭当前文件
        {
            let mut file_lock = self.current_file.lock().unwrap();
            *file_lock = None;
        }
        {
            let mut size_lock = self.current_size.lock().unwrap();
            *size_lock = 0;
        }
        // 检查文件是否存在
        if !self.path.exists() {
            return Ok(());
        }
        // 生成滚动文件名
        let now = Local::now();
        let timestamp = now.format("%Y%m%d-%H%M%S");
        let stem = self.path.file_stem().and_then(|s| s.to_str()).unwrap_or("log");
        let ext = self.path.extension().and_then(|s| s.to_str()).unwrap_or("log");
        let rotated_path = self.path.with_file_name(format!(
            "{}.{}.1.{}",
            stem,
            timestamp,
            ext
        ));
        // 重命名当前文件
        fs::rename(&self.path, &rotated_path)?;
        // 删除旧滚动文件
        self.clean_old_files()?;
        Ok(())
    }

    fn clean_old_files(&self) -> io::Result<()> {
        let stem = self.path.file_stem().and_then(|s| s.to_str()).unwrap_or("log");
        let ext = self.path.extension().and_then(|s| s.to_str()).unwrap_or("log");
        let dir = self.path.parent().unwrap_or(std::path::Path::new("."));

        let mut files: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with(stem) && name_str.ends_with(&format!(".{}", ext))
            })
            .collect();

        // 按修改时间排序（最旧的在前）
        files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

        // 删除超过 max_files 的旧文件
        while files.len() > self.max_files {
            if let Some(oldest) = files.first() {
                let _ = fs::remove_file(oldest.path());
                files.remove(0);
            }
        }

        Ok(())
    }
}

impl Write for SizeRotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = self.get_or_create_file()?;
        let result = file.write(buf);
        let written = match result {
            Ok(n) => {
                let mut size_lock = self.current_size.lock().unwrap();
                *size_lock += n as u64;
                n
            }
            Err(e) => return Err(e),
        };
        let mut file_lock = self.current_file.lock().unwrap();
        *file_lock = Some(file);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file_lock = self.current_file.lock().unwrap();
        if let Some(ref mut file) = *file_lock {
            file.flush()?;
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TraceID
// ─────────────────────────────────────────────────────────────────────────────

thread_local! {
    static TRACE_ID: Mutex<Option<String>> = Mutex::new(None);
}

/// 生成新的 TraceID
pub fn generate_trace_id() -> String {
    Uuid::new_v4().to_string()
}

/// 从请求头获取或生成 TraceID
pub fn extract_or_create_trace_id(header: &str) -> String {
    if header.is_empty() {
        generate_trace_id()
    } else {
        header.to_string()
    }
}

/// 设置当前线程的 TraceID
pub fn set_trace_id(trace_id: String) {
    TRACE_ID.with(|cell| {
        let mut id = cell.lock().unwrap();
        *id = Some(trace_id);
    });
}

/// 获取当前线程的 TraceID
pub fn get_trace_id() -> Option<String> {
    TRACE_ID.with(|cell| cell.lock().unwrap().clone())
}

/// 清除当前线程的 TraceID
pub fn clear_trace_id() {
    TRACE_ID.with(|cell| {
        let mut id = cell.lock().unwrap();
        *id = None;
    });
}

/// 获取当前时间戳（毫秒）
pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────────────
// 脱敏
// ─────────────────────────────────────────────────────────────────────────────

/// 需要脱敏的敏感字段名
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "access_token",
    "refresh_token",
    "authorization",
    "token",
    "code",
    "plaintext_code",
    "code_ciphertext",
    "code_nonce",
    "payment_note",
    "email",
    "secret",
    "api_key",
    "private_key",
];

/// 对值进行脱敏
pub fn redact_value(key: &str, value: &str) -> String {
    let key_lower = key.to_lowercase();

    // 先检查字段名是否包含 email 关键词（邮箱需要特殊处理）
    if key_lower.contains("email") {
        if let Some(at_pos) = value.find('@') {
            let domain = &value[at_pos..];
            format!("***@{}", domain)
        } else {
            let hash = hash_bytes(value.as_bytes());
            format!("***REDACTED({})***", &hash[..8])
        }
    }
    // 检查是否是完全匹配的敏感字段
    else if SENSITIVE_KEYS.iter().any(|k| *k == key_lower) {
        let hash = hash_bytes(value.as_bytes());
        format!("***REDACTED({})***", &hash[..12])
    }
    // UUID 直接记录（不脱敏）
    else if is_uuid(value) {
        value.to_string()
    }
    // 默认返回原值
    else {
        value.to_string()
    }
}

/// 检查字符串是否为 UUID
fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// 对 JSON 对象进行脱敏
pub fn redact_json(json: &serde_json::Value) -> serde_json::Value {
    match json {
        serde_json::Value::Object(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                let key_lower = key.to_lowercase();
                // 先检查 email 关键词
                if key_lower.contains("email") {
                    if let Some(email_str) = value.as_str() {
                        if let Some(at_pos) = email_str.find('@') {
                            let domain = &email_str[at_pos..];
                            result.insert(key.clone(), serde_json::Value::String(format!("***@{}", domain)));
                        } else {
                            result.insert(key.clone(), value.clone());
                        }
                    } else {
                        result.insert(key.clone(), value.clone());
                    }
                }
                // 检查完全匹配的敏感字段
                else if SENSITIVE_KEYS.iter().any(|k| *k == key_lower.as_str()) {
                    let hash = hash_bytes(value.to_string().as_bytes());
                    result.insert(
                        key.clone(),
                        serde_json::Value::String(format!("***REDACTED({})***", &hash[..8])),
                    );
                }
                // 递归处理嵌套对象
                else {
                    result.insert(key.clone(), redact_json(value));
                }
            }
            serde_json::Value::Object(result)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redact_json).collect())
        }
        _ => json.clone(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 辅助函数
// ─────────────────────────────────────────────────────────────────────────────

/// 计算 SHA256 哈希
pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// 计算字符串的哈希
pub fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

/// 计算文件哈希（用于上传摘要）
pub fn file_hash(bytes: &[u8]) -> String {
    format!("sha256:{}", &hash_bytes(bytes)[..16])
}

// ─────────────────────────────────────────────────────────────────────────────
// DB 审计日志
// ─────────────────────────────────────────────────────────────────────────────

/// 记录 DB 操作
pub fn db_operation(
    operation: &'static str,
    sql_kind: &str,
    duration_ms: u64,
    rows_affected: i64,
    params: Option<&serde_json::Value>,
) {
    let sql_hash = hash_text(sql_kind);

    if let Some(p) = params {
        tracing::debug!(
            target: "db",
            event = "db.execute",
            db_op = operation,
            sql_kind = sql_kind,
            sql_hash = %sql_hash,
            params = %p,
            rows_affected = rows_affected,
            duration_ms = duration_ms,
        );
    } else {
        tracing::debug!(
            target: "db",
            event = "db.execute",
            db_op = operation,
            sql_kind = sql_kind,
            sql_hash = %sql_hash,
            rows_affected = rows_affected,
            duration_ms = duration_ms,
        );
    }
}

/// 记录 DB 查询结果摘要
pub fn db_query_result(
    operation: &'static str,
    duration_ms: u64,
    rows: usize,
    summary: Option<&serde_json::Value>,
) {
    tracing::debug!(
        target: "db",
        event = "db.query.end",
        db_op = operation,
        rows = rows,
        duration_ms = duration_ms,
        record_summary = ?summary,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 安全日志
// ─────────────────────────────────────────────────────────────────────────────

/// 记录安全事件
pub fn security_event(
    event_type: &str,
    message: &str,
    details: Option<&serde_json::Value>,
) {
    if let Some(d) = details {
        tracing::warn!(
            target: "security",
            event = format!("security.{}", event_type),
            message = %message,
            details = %d,
        );
    } else {
        tracing::warn!(
            target: "security",
            event = format!("security.{}", event_type),
            message = %message,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TraceID HTTP 中间件
// ─────────────────────────────────────────────────────────────────────────────

use std::time::Instant;

/// TraceID 中间件层
#[derive(Clone)]
pub struct TraceIdLayer {
    config: LogConfig,
}

impl TraceIdLayer {
    pub fn new() -> Self {
        Self {
            config: LogConfig::from_env(),
        }
    }
}

impl Default for TraceIdLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for TraceIdLayer {
    type Service = TraceIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceIdMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// TraceID 中间件服务
#[derive(Clone)]
pub struct TraceIdMiddleware<S> {
    inner: S,
    config: LogConfig,
}

impl<S> TraceIdMiddleware<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            config: LogConfig::from_env(),
        }
    }
}

impl<S, B> Service<Request<B>> for TraceIdMiddleware<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static + HttpBody,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn Send + std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let config = self.config.clone();
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            // 提取或生成 TraceID
            let trace_id = extract_trace_id_from_request(&request, &config.trace_header);

            // 记录请求开始
            let method = request.method().to_string();
            let path = request.uri().path().to_string();
            let start_time = Instant::now();

            // 获取远程地址
            let remote_addr = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // 获取 User-Agent
            let user_agent = request
                .headers()
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            // 估算请求体大小
            let request_bytes = request
                .body()
                .size_hint()
                .upper()
                .unwrap_or(0);

            tracing::info!(
                target: "api",
                event = "api.request.start",
                trace_id = %trace_id,
                method = %method,
                path = %path,
                remote_addr = %remote_addr,
                user_agent = %user_agent,
                request_bytes = %request_bytes,
            );

            // 设置 TraceID 到线程本地存储
            set_trace_id(trace_id.clone());

            // 添加 TraceID 到请求扩展
            let mut request = request;
            request.extensions_mut().insert(TraceId(trace_id.clone()));

            // 调用内部服务
            let response = inner.call(request).await?;

            // 计算耗时
            let duration_ms = start_time.elapsed().as_millis() as u64;

            // 获取响应状态码和大小
            let status = response.status().as_u16();
            let response_bytes = response
                .body()
                .size_hint()
                .upper()
                .unwrap_or(0);

            // 记录请求结束
            tracing::info!(
                target: "api",
                event = "api.request.end",
                trace_id = %trace_id,
                method = %method,
                path = %path,
                status = %status,
                duration_ms = %duration_ms,
                response_bytes = %response_bytes,
            );

            // 清除 TraceID
            clear_trace_id();

            // 在响应头中添加 TraceID
            let mut response = response;
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::try_from(config.trace_header.as_str()),
                HeaderValue::from_str(&trace_id),
            ) {
                response.headers_mut().insert(header_name, header_value);
            }

            Ok(response)
        })
    }
}

/// 从请求中提取 TraceID
fn extract_trace_id_from_request<B>(request: &Request<B>, header_name: &str) -> String {
    // 先检查自定义 header
    if let Some(trace_id) = request.headers().get(header_name) {
        if let Ok(id) = trace_id.to_str() {
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }

    // 检查 W3C traceparent header
    if let Some(traceparent) = request.headers().get("traceparent") {
        if let Ok(tp) = traceparent.to_str() {
            // traceparent 格式: 00-{trace_id}-{span_id}-{flags}
            if let Some(trace_id) = tp.split('-').nth(1) {
                if !trace_id.is_empty() {
                    return trace_id.to_string();
                }
            }
        }
    }

    // 生成新的 UUID
    generate_trace_id()
}

/// TraceID 扩展类型
#[derive(Debug, Clone)]
pub struct TraceId(pub String);

impl TraceId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_password() {
        let result = redact_value("password", "secret123");
        assert!(result.starts_with("***REDACTED("));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_redact_email() {
        let result = redact_value("email", "user@example.com");
        assert!(result.contains("@example.com"));
        assert!(!result.contains("user@"));
    }

    #[test]
    fn test_redact_token() {
        let result = redact_value("access_token", "eyJhbGciOiJIUzI1NiJ9...");
        assert!(result.starts_with("***REDACTED("));
        assert!(!result.contains("eyJ"));
    }

    #[test]
    fn test_uuid_not_redacted() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let result = redact_value("user_id", uuid);
        assert_eq!(result, uuid);
    }

    #[test]
    fn test_hash_bytes() {
        let hash1 = hash_bytes(b"hello");
        let hash2 = hash_bytes(b"hello");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_redact_json() {
        let json = serde_json::json!({
            "email": "user@example.com",
            "password": "secret",
            "name": "John"
        });
        let redacted = redact_json(&json);
        let map = redacted.as_object().unwrap();
        assert!(map.get("email").unwrap().as_str().unwrap().contains("@"));
        assert!(map.get("password").unwrap().as_str().unwrap().starts_with("***"));
        assert_eq!(map.get("name").unwrap().as_str().unwrap(), "John");
    }

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36);
    }

    #[test]
    fn test_trace_id_thread_local() {
        let id = generate_trace_id();
        set_trace_id(id.clone());
        assert_eq!(get_trace_id(), Some(id.clone()));
        clear_trace_id();
        assert_eq!(get_trace_id(), None);
    }
}
