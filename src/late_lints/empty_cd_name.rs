use ariadne::{Color, Label, Report, ReportKind};
use tracing::*;

use crate::config::Config;
use crate::custom_difficulty::CustomDifficulty;

pub fn lint_empty_cd_name<'a>(
    _config: &Config,
    cd: &CustomDifficulty,
    path: &'a String,
    diag: &mut Vec<Report<'a, (&'a String, std::ops::Range<usize>)>>,
) {
    debug!("{:#?}", cd);

    if cd.name.val.is_empty() {
        debug!(cd_name_span = ?cd.name.span);
        diag.push(
            Report::build(ReportKind::Warning, path, cd.name.span.start)
                .with_message("custom difficulty name is empty")
                .with_label(Label::new((path, cd.name.span.into_range())).with_color(Color::Yellow))
                .finish(),
        );
    }
}
