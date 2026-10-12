use std::path::Path;

use crate::error::SysError;

/// Read a file as UTF-8 text.
///
/// NOTE: path traversal is NOT blocked here. `policy` decides what
/// may be read; this function just performs the read.
pub async fn read_text(path: &Path) -> Result<String, SysError> {
    Ok(tokio::fs::read_to_string(path).await?)
}
