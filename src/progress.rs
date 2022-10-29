use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::borrow::Cow;

const TEMPLATE: &str = "{spinner:.green} {prefix:.magenta}: [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}";

pub fn new_progress_bar(
    multi_progress: &MultiProgress,
    name: impl Into<Cow<'static, str>>,
    size: u64,
) -> ProgressBar {
    let bar = multi_progress.add(ProgressBar::new(size));
    bar.set_style(
        ProgressStyle::with_template(TEMPLATE)
            .unwrap()
            .progress_chars("#>-"),
    );
    bar.set_prefix(name);
    bar
}
