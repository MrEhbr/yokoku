use std::path::PathBuf;

use rstest::rstest;
use yokoku_domain::{InvalidFolderName, ItemFolder};

#[rstest]
#[case::named("Frieren (2023)")]
#[case::dotted("Mr. Robot")]
fn a_single_component_is_a_folder_name(#[case] name: &str) {
    let folder = ItemFolder::new(PathBuf::from("/tv"), name.into()).unwrap();

    assert_eq!(folder.path(), PathBuf::from("/tv").join(name));
}

#[rstest]
#[case::empty("")]
#[case::nested("Frieren/Season 1")]
#[case::trailing_separator("Frieren/")]
#[case::current(".")]
#[case::parent("..")]
#[case::absolute("/Frieren")]
fn anything_else_is_rejected(#[case] name: &str) {
    assert_eq!(ItemFolder::new(PathBuf::from("/tv"), name.into()), Err(InvalidFolderName(name.into())));
}
