use tempfile::TempDir;

pub fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}
