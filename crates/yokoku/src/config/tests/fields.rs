use crate::config::Config;

#[test]
fn every_field_on_the_settings_page_is_a_setting_the_database_can_store() {
    for field in Config::fields() {
        if let Err(error) = Config::editable(&field.key) {
            panic!("{}: {error:#}", field.key);
        }
    }
}
