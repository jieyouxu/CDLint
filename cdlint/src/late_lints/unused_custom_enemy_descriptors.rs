use std::collections::BTreeMap;

use ariadne::{Color, Fmt, Label, Report, ReportKind};

use crate::config::Config;
use crate::custom_difficulty::{CustomDifficulty, EnemyPool};
use crate::spanned::Spanned;
use crate::Diagnostics;

use super::VANILLA_ENEMY_DESCRIPTORS;

pub fn lint_unused_custom_enemy_descriptors<'d>(
    config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) {
    let mut custom_descriptors_usage = BTreeMap::new();
    for ed_name in cd.enemy_descriptors.val.keys() {
        if !VANILLA_ENEMY_DESCRIPTORS.contains(&ed_name.val.as_str())
            && !config.extra_enemy_descriptors.contains(&ed_name.val)
        {
            custom_descriptors_usage.insert(ed_name.val.to_owned(), (ed_name.span, false));
        }
    }

    let mut update_usage = |enemy_pool: &Spanned<EnemyPool>| {
        let mut update = |target: &Spanned<Vec<Spanned<String>>>| {
            for name in &target.val {
                custom_descriptors_usage
                    .entry(name.val.to_owned())
                    .and_modify(|(_, is_used)| *is_used = true);
            }
        };

        update(&enemy_pool.val.add);
        update(&enemy_pool.val.remove);
    };

    update_usage(&cd.enemy_pool);
    update_usage(&cd.common_enemies);
    update_usage(&cd.disruptive_enemies);
    update_usage(&cd.special_enemies);
    update_usage(&cd.stationary_enemies);

    custom_descriptors_usage
        .iter()
        .filter(|(_, (_, usage))| !(*usage))
        .for_each(|(name, (span, _))| {
            diag.push(
                Report::build(ReportKind::Warning, path, span.start)
                    .with_message(format!(
                        "custom Enemy Descriptor \"{}\" is defined but never used",
                        name.fg(Color::Blue)
                    ))
                    .with_label(
                        Label::new((path, span.into_range()))
                            .with_color(Color::Yellow)
                            .with_message(format!("\"{}\" is defined here", name.fg(Color::Blue))),
                    )
                    .finish(),
            );
        });
}
