use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn fixture_path(relative: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn read_fixture(relative: impl AsRef<Path>) -> String {
    fs::read_to_string(fixture_path(relative)).expect("fixture file should be readable")
}

#[allow(dead_code)]
pub fn edgar() -> edgarkit::Edgar {
    edgarkit::Edgar::new("test_agent example@example.com").unwrap()
}
