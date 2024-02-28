use ariadne::{Color, Fmt, Label, Report, ReportKind};

use crate::config::Config;
use crate::custom_difficulty::{CustomDifficulty, EnemyPool};
use crate::spanned::Spanned;
use crate::Diagnostics;

pub fn lint_ambiguous_enemy_pool_add_remove<'d>(
    _config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) {
    let check_enemy_pool = |diag: &mut Diagnostics<'d>, pool: &Spanned<EnemyPool>| {
        for Spanned {
            val: add_name,
            span: add_span,
        } in pool.val.add.val.iter()
        {
            if let Some(Spanned {
                val: remove_name,
                span: remove_span,
            }) = pool
                .val
                .remove
                .val
                .iter()
                .find(|remove_name| &remove_name.val == add_name)
            {
                let add_label = Label::new((path, add_span.into_range()))
                    .with_color(Color::Yellow)
                    .with_message(format!("\"{}\" appears here", add_name.fg(Color::Blue)));
                let remove_label = Label::new((path, remove_span.into_range()))
                    .with_color(Color::Yellow)
                    .with_message(format!(
                        "\"{}\" also appears here",
                        remove_name.fg(Color::Blue)
                    ));

                diag.push(
                    Report::build(ReportKind::Warning, path, add_span.start)
                        .with_message(format!("ambiguous Enemy Descriptor addition/removal from enemy pool: \"{}\" appears in both \"{}\" and \"{}\"", add_name.fg(Color::Blue), "add".fg(Color::Blue), "remove".fg(Color::Blue)))
                        .with_label(add_label)
                        .with_label(remove_label)
                        .with_help(format!("consider removing \"{}\" from one of the array", add_name.fg(Color::Blue)))
                        .finish(),
                );
            }
        }
    };

    check_enemy_pool(diag, &cd.enemy_pool);
    check_enemy_pool(diag, &cd.common_enemies);
    check_enemy_pool(diag, &cd.disruptive_enemies);
    check_enemy_pool(diag, &cd.special_enemies);
    check_enemy_pool(diag, &cd.stationary_enemies);
}
