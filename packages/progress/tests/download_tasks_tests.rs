use progresshub_progress::orchestration::download_tasks::*;
use progresshub_progress::{DownloadConfig, RepoManifest};

#[tokio::test]
async fn test_empty_manifest_execution() {
    let manifest = RepoManifest {
        repo_id: "test/repo".to_string(),
        files: Vec::new(),
        total_size: 0,
    };
    let config = DownloadConfig::default();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let results = execute_downloads("test/repo", &manifest, temp_dir.path(), &config, None)
        .await
        .expect("Should handle empty manifest");

    assert!(results.is_empty());
}
