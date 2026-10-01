use yokoku_web::Control;

use super::keys;
use crate::config::Config;

#[test]
fn every_editable_field_on_the_settings_page_is_a_setting_the_database_can_store() {
    for field in Config::fields().into_iter().filter(|field| field.control != Control::ReadOnly) {
        if let Err(error) = Config::editable(&field.key) {
            panic!("{}: {error:#}", field.key);
        }
    }
}

#[test]
fn every_read_only_field_is_a_setting_the_database_cannot_store() {
    for field in Config::fields().into_iter().filter(|field| field.control == Control::ReadOnly) {
        let config = Config::default();
        assert!(config.value(&field.key).is_ok(), "{} is not a setting", field.key);
        assert!(Config::editable(&field.key).is_err(), "{} can be stored", field.key);
    }
}

#[test]
fn every_setting_is_on_the_settings_page() {
    let shown: Vec<String> = Config::fields().into_iter().map(|field| field.key).collect();

    for key in keys() {
        assert!(shown.contains(&key), "{key} is not on the settings page");
    }
}
