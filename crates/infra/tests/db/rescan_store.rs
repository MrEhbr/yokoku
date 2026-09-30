use jiff::Timestamp;
use rstest::rstest;
use support::db;
use yokoku_infra::db::Database;
use yokoku_integrations::ports::RescanStore;

use crate::support;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(1_790_000_000 + second).unwrap()
}

#[rstest]
#[tokio::test]
async fn a_request_keeps_the_later_time(#[future(awt)] db: Database) {
    let nothing = db.requested_at().await.unwrap();

    db.request(at(2)).await.unwrap();
    db.request(at(1)).await.unwrap();

    assert_eq!((nothing, db.requested_at().await.unwrap()), (None, Some(at(2))));
}

#[rstest]
#[tokio::test]
async fn clearing_removes_only_the_request_made_then(#[future(awt)] db: Database) {
    db.request(at(1)).await.unwrap();

    db.clear(at(0)).await.unwrap();
    let renewed = db.requested_at().await.unwrap();
    db.clear(at(1)).await.unwrap();

    assert_eq!((renewed, db.requested_at().await.unwrap()), (Some(at(1)), None));
}
