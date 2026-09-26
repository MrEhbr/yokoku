use proptest::prelude::*;
use yokoku_domain::ImportId;

proptest! {
    #[test]
    fn ids_parse_back_from_display(value: u128) {
        let id = ImportId(uuid::Uuid::from_u128(value));
        prop_assert_eq!(id.to_string().parse::<ImportId>().ok(), Some(id));
    }
}

#[test]
fn ids_reject_text_that_is_not_a_uuid() {
    assert!("0190-not-an-id".parse::<ImportId>().is_err());
}
