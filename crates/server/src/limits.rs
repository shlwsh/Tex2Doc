//! 请求体大小限制常量。

/// 单请求最大字节数（50 MiB）。
pub const MAX_BODY: usize = 50 * 1024 * 1024;

/// 单个上传 ZIP 最大字节数（50 MiB）。
pub const MAX_UPLOAD_ZIP_BYTES: usize = MAX_BODY;

/// ZIP 解压后的最大总字节数（200 MiB）。
pub const MAX_UPLOAD_UNCOMPRESSED_BYTES: u64 = 200 * 1024 * 1024;

/// ZIP 内单文件最大字节数（50 MiB）。
pub const MAX_UPLOAD_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// ZIP 内最多文件数。
pub const MAX_UPLOAD_FILE_COUNT: usize = 2_000;
