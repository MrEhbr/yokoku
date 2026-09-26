use serde_json::json;
use yokoku_db::Database;

#[tokio::test]
async fn settings_are_stored_replaced_and_removed_by_key() {
    let db = Database::open_in_memory().await.unwrap();

    db.set_setting("recycle.keep_days", &json!(14)).await.unwrap();
    db.set_setting("import.mode", &json!("copy")).await.unwrap();
    db.set_setting("recycle.keep_days", &json!(7)).await.unwrap();
    let stored = db.settings().await.unwrap();
    let removed = db.remove_setting("import.mode").await.unwrap();
    let removed_again = db.remove_setting("import.mode").await.unwrap();

    assert_eq!(stored, [("import.mode".to_owned(), json!("copy")), ("recycle.keep_days".to_owned(), json!(7))]);
    assert!(removed && !removed_again);
    assert_eq!(db.settings().await.unwrap(), [("recycle.keep_days".to_owned(), json!(7))]);
}
