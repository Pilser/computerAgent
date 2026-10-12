use std::path::Path;

use crate::error::SysError;

/// Write UTF-8 text to a file, creating or truncating it.
///
/// NOTE: path traversal is NOT blocked here. `policy` decides what
/// may be written; this function just performs the write.
pub async fn write_text(path: &Path, contents: &str) -> Result<(), SysError> {
    Ok(tokio::fs::write(path, contents.as_bytes()).await?)
}
