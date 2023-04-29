use std::iter::repeat;
use std::path::{Path, PathBuf};

use base64::engine::GeneralPurpose;


#[macro_export]
macro_rules! test_file {
    ($path: expr, $file: literal) => {
        $path.join($file).exists()
    };
}

pub fn find_first_subpath<P: AsRef<Path>, F: Fn(&Path) -> bool>(
    root: impl AsRef<Path>,
    subpaths: &[P],
    search: F,
) -> Option<PathBuf> {
    subpaths
        .iter()
        .zip(repeat(root.as_ref()))
        .map(|(b, a)| a.join(b))
        .find(|it: &PathBuf| search(&it))
}

pub fn base64_engine() -> GeneralPurpose {
    base64::engine::GeneralPurpose::new(
        &base64::alphabet::URL_SAFE,
        base64::engine::GeneralPurposeConfig::new(),
    )
}
