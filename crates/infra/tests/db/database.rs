use yokoku_infra::db::Database;

#[tokio::test]
async fn open_creates_the_database_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data/nested/yokoku.db");

    Database::open(&path).await.unwrap();

    assert!(path.is_file());
}
