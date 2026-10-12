pub mod read;
pub mod write;

pub use read::read_text;
pub use write::write_text;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn fs_round_trip_in_opencode_tmp() {
        let dir: PathBuf =
            std::path::Path::new("/tmp/opencode").join(format!("sys-tools-{}", std::process::id()));
        let file = dir.join("roundtrip.txt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_text(&file, "hello sys-tools").await.unwrap();
        let back = read_text(&file).await.unwrap();
        assert_eq!(back, "hello sys-tools");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
