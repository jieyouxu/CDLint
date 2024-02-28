use ariadne::{Color, Fmt, Label, Report, ReportKind};

use crate::config::Config;
use crate::custom_difficulty::{CustomDifficulty, Range, WeightedRange};
use crate::spanned::Spanned;
use crate::Diagnostics;

/// This lint goes through all `Range`s and `WeightedRange`s (by implication) to find any cases
/// where `min > max`. This is extremely confusing, and its behavior in Custom Difficulty and in
/// game isn't very clear or obvious.
pub fn lint_min_larger_than_max<'d>(
    _config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) {
    let weighted_int_range_check =
        |diag: &mut Diagnostics<'d>, r: &Spanned<WeightedRange<usize>>| {
            let Spanned {
                val: weighted_range,
                ..
            } = &r;
            let Range { min, max } = &weighted_range.range.val;

            if min.val > max.val {
                diag.push(
               Report::build(ReportKind::Warning, path, weighted_range.range.span.start)
                   .with_message(format!("{} in this range, which may lead to surprising behavior in Custom Difficulty and in game", "min > max".fg(Color::Blue)))
                   .with_label(
                       Label::new((path, min.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .with_label(
                       Label::new((path, max.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .finish(),
           );
            }
        };

    let weighted_float_range_check =
        |diag: &mut Diagnostics<'d>, r: &Spanned<WeightedRange<f64>>| {
            let Spanned {
                val: weighted_range,
                ..
            } = &r;
            let Range { min, max } = &weighted_range.range.val;

            if min.val > max.val {
                diag.push(
               Report::build(ReportKind::Warning, path, weighted_range.range.span.start)
                   .with_message(format!("{} in this range, which may lead to surprising behavior in Custom Difficulty and in game", "min > max".fg(Color::Blue)))
                   .with_label(
                       Label::new((path, min.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .with_label(
                       Label::new((path, max.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .finish(),
           );
            }
        };

    let int_range_check = |diag: &mut Diagnostics<'d>, r: &Spanned<Range<usize>>| {
        let Spanned {
            val: Range { min, max },
            ..
        } = &r;

        if min.val > max.val {
            diag.push(
               Report::build(ReportKind::Warning, path, r.span.start)
                   .with_message(format!("{} in this range, which may lead to surprising behavior in Custom Difficulty and in game", "min > max".fg(Color::Blue)))
                   .with_label(
                       Label::new((path, min.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .with_label(
                       Label::new((path, max.span.into_range()))
                           .with_color(Color::Yellow),
                   )
                   .finish(),
           );
        }
    };

    cd.encounter_difficulty
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.stationary_difficulty
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.enemy_wave_interval
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.enemy_normal_wave_difficulty
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.enemy_diversity
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.stationary_enemy_diversity
        .val
        .iter()
        .for_each(|r| weighted_int_range_check(diag, r));
    cd.veteran_normal
        .val
        .iter()
        .for_each(|r| weighted_float_range_check(diag, r));
    cd.veteran_large
        .val
        .iter()
        .for_each(|r| weighted_float_range_check(diag, r));
    int_range_check(diag, &cd.disruptive_enemy_pool_count);
}
