use std::collections::HashSet;

use ariadne::{Color, Fmt, Label, Report, ReportKind};

use crate::config::Config;
use crate::custom_difficulty::CustomDifficulty;
use crate::late_lints::VANILLA_ENEMY_DESCRIPTORS;
use crate::spanned::Spanned;
use crate::Diagnostics;

pub fn lint_undefined_enemy_descriptors<'d>(
    config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) {
    let mut defined_enemy_descriptors = HashSet::new();
    defined_enemy_descriptors.extend(VANILLA_ENEMY_DESCRIPTORS.into_iter().map(ToOwned::to_owned));
    defined_enemy_descriptors.extend(config.extra_enemy_descriptors.iter().map(ToOwned::to_owned));

    // 1. First, we collect any custom defined Enemy Descriptors.
    for (ed_name, ed_def) in &cd.enemy_descriptors.val {
        if !defined_enemy_descriptors.contains(&ed_name.val) {
            if ed_def.val.base.val == ed_name.val {
                // We're referencing ourselves, but we haven't defined it yet!
                diag.push(
                    Report::build(ReportKind::Error, path, ed_name.span.start)
                        .with_message(format!("attempt to reference \"{}\" in its \"Base\" field that is not a pre-defined Enemy Descriptor", ed_name.val.as_str().fg(Color::Blue)))
                        .with_label(
                            Label::new((path, ed_name.span.into_range())).with_color(Color::Red),
                        )
                        .finish(),
                );
            } else {
                defined_enemy_descriptors.insert(ed_name.val.to_owned());
            }
        } else if !defined_enemy_descriptors.contains(&ed_def.val.base.val) {
            diag.push(
                Report::build(ReportKind::Error, path, ed_def.val.base.span.start)
                    .with_message(format!(
                        "attempt to reference undefined Enemy Descriptor \"{}\" as \"Base\"",
                        ed_def.val.base.val.as_str().fg(Color::Blue)
                    ))
                    .with_label(Label::new((path, ed_def.span.into_range())).with_color(Color::Red))
                    .finish(),
            );
        }
    }

    let mut check_ed = |ed: &Spanned<String>| {
        if !defined_enemy_descriptors.contains(&ed.val) {
            diag.push(
                Report::build(ReportKind::Error, path, ed.span.start)
                    .with_message(format!(
                        "attempt to reference undefined Enemy Descriptor \"{}\"",
                        ed.val.as_str().fg(Color::Blue)
                    ))
                    .with_label(Label::new((path, ed.span.into_range())).with_color(Color::Red))
                    .finish(),
            );
        }
    };

    // 2. Now, we need to check each of the enemy pool's add/remove members to see if they attempt
    //    to reference undefined enemy descriptors.
    cd.enemy_pool.val.add.val.iter().for_each(&mut check_ed);
    cd.enemy_pool.val.remove.val.iter().for_each(&mut check_ed);

    cd.common_enemies.val.add.val.iter().for_each(&mut check_ed);
    cd.common_enemies
        .val
        .remove
        .val
        .iter()
        .for_each(&mut check_ed);

    cd.disruptive_enemies
        .val
        .add
        .val
        .iter()
        .for_each(&mut check_ed);
    cd.disruptive_enemies
        .val
        .remove
        .val
        .iter()
        .for_each(&mut check_ed);

    cd.special_enemies
        .val
        .add
        .val
        .iter()
        .for_each(&mut check_ed);
    cd.special_enemies
        .val
        .remove
        .val
        .iter()
        .for_each(&mut check_ed);

    cd.stationary_enemies
        .val
        .add
        .val
        .iter()
        .for_each(&mut check_ed);
    cd.stationary_enemies
        .val
        .remove
        .val
        .iter()
        .for_each(&mut check_ed);
}
