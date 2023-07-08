use std::borrow::Cow;

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;

static PROGRESS_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    ProgressStyle::with_template(&get_template())
        .unwrap()
        .progress_chars("━━")
});

pub fn new_progress_bar(
    multi_progress: &MultiProgress,
    name: impl Into<Cow<'static, str>>,
    size: u64,
) -> ProgressBar {
    let bar = multi_progress.add(ProgressBar::new(size));
    bar.set_style(PROGRESS_STYLE.clone());
    bar.set_prefix(name);
    bar
}

const PREFIX: &str = "{spinner:.green} {prefix}:";
const PROGRESS: &str = "{bar:40.red/white}";
const SUFFIX: &str = "{msg}";

fn get_template() -> String {
    let time = style("[").cyan().to_string()
        + "{elapsed:.cyan}"
        + &style("]").cyan().to_string();

    let position = String::from("{pos:.green}")
        + &style("/").green().to_string()
        + &String::from("{len:.green}");

    let template = format!("{PREFIX} {time} {PROGRESS} {position} {SUFFIX}");
    template
}
