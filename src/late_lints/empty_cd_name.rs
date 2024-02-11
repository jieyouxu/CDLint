use ariadne::{Color, Label, Report, ReportKind};

use crate::config::Config;
use crate::custom_difficulty::CustomDifficulty;
use crate::Diagnostics;

pub fn lint_empty_cd_name<'d>(
    _config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) {
    if cd.name.val.is_empty() {
        diag.push(
            Report::build(ReportKind::Warning, path, cd.name.span.start)
                .with_message("custom difficulty name is empty")
                .with_label(Label::new((path, cd.name.span.into_range())).with_color(Color::Yellow))
                .finish(),
        );
    }
}
