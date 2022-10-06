use std::iter::{repeat, Repeat};
use std::path::{Path, PathBuf};

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
        .find(|it| search(&it))
}
