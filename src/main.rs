#![feature(let_chains)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Context};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use custom_difficulty::ArrayOrSingleItem;
use serde::Deserialize;
use tracing::*;

use crate::custom_difficulty::CustomDifficulty;
use crate::parser::Json;

mod custom_difficulty;
mod edit_distance;
mod late_lints;
mod logging;
mod parser;

#[derive(Debug, ClapParser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to a Custom Difficulty JSON file.
    input: PathBuf,
}

#[derive(Debug, Eq, Deserialize, Clone)]

pub struct Spanned<T> {
    #[serde(skip_serializing)]
    pub span: SimpleSpan<usize>,
    pub val: T,
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.val == other.val
    }
}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.val.partial_cmp(&other.val)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.val.cmp(&other.val)
    }
}

fn main() -> anyhow::Result<()> {
    logging::setup_logging();

    let cli = Args::parse();

    debug!(input = ?cli.input);

    let json_string = match std::fs::read_to_string(&cli.input) {
        Ok(file) => file,
        Err(e) => {
            error!(path = ?cli.input, "failed to read input");
            return Err(e)
                .with_context(|| format!("failed to read file `{}`", cli.input.display()));
        }
    };

    let path = cli.input.display().to_string();

    let (custom_difficulty_json, errors) =
        parser::parser().parse(&json_string).into_output_errors();

    errors.into_iter().for_each(|e| {
        Report::build(ReportKind::Error, &path, e.span().start)
            .with_message(e.to_string())
            .with_label(
                Label::new((&path, e.span().into_range()))
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .print((&path, Source::from(&json_string)))
            .unwrap()
    });

    let Some(custom_difficulty_json) = custom_difficulty_json else {
        bail!("failed to parse Custom Difficulty JSON");
    };

    debug!(?custom_difficulty_json);

    let Spanned {
        val: Json::Object(Spanned {
            val: top_level_members,
            ..
        }),
        ..
    } = custom_difficulty_json
    else {
        bail!("unexpected top level JSON kind");
    };

    let mut diagnostics = Vec::new();
    let mut custom_difficulty = CustomDifficulty::default();

    // There are two kinds of lints:
    // 1. Early-pass lints: these lints are performed while parsing the CD JSON into the CD struct.
    // 2. Late-pass lints: these lints are performed on the built CD struct.

    handle_top_level_members(
        &mut diagnostics,
        &path,
        &json_string,
        &mut custom_difficulty,
        &top_level_members,
    )
    .context("trying to process top level members")?;

    late_lints::lint_empty_cd_name(&custom_difficulty, &path, &mut diagnostics);

    for diagnostic in diagnostics {
        diagnostic.print((&path, Source::from(&json_string)))?;
    }

    Ok(())
}

fn handle_str<'d, 'a>(
    diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<String>,
    member_val: &Spanned<Json>,
    member_name: &'static str,
) -> anyhow::Result<()> {
    *target = if let Json::Str(s) = &member_val.val {
        s.to_owned()
    } else {
        unexpected_value_kind(path, member_val, "string").print((path, Source::from(src)))?;
        bail!("unexpected JSON kind found in \"{member_name}\" member value");
    };
    Ok(())
}

fn handle_single_item_or_array<T>(
    diag: &mut Diagnostics<'_>,
    target: &mut Spanned<Json>,
    member_val: &Spanned<Json>,
    handle_elem: impl FnMut(),
) -> anyhow::Result<()> {
    todo!()
}

fn handle_top_level_members<'d, 'a>(
    diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    cd: &mut CustomDifficulty,
    top_level_members: &BTreeMap<Spanned<String>, Spanned<Json>>,
) -> anyhow::Result<()> {
    for (member_name, member_val) in top_level_members {
        match member_name.val.as_str() {
            "Name" => handle_str(diag, path, src, &mut cd.name, member_val, "Name")?,
            "Description" => handle_str(
                diag,
                path,
                src,
                &mut cd.description,
                member_val,
                "Description",
            )?,
            "MaxActiveCritters" => todo!(),
            "MaxActiveSwarmers" => todo!(),
            "MaxActiveEnemies" => todo!(),
            "ResupplyCost" => todo!(),
            "StartingNitra" => todo!(),
            "ExtraLargeEnemyDamageResistance" => todo!(),
            "ExtraLargeEnemyDamageResistanceB" => todo!(),
            "ExtraLargeEnemyDamageResistanceC" => todo!(),
            "ExtraLargeEnemyDamageResistanceD" => todo!(),
            "EnemyDamageResistance" => todo!(),
            "SmallEnemyDamageResistance" => todo!(),
            "EnemyDamageModifier" => todo!(),
            "EnemyCountModifier" => todo!(),
            "EncounterDifficulty" => todo!(),
            "StationaryDifficulty" => todo!(),
            "EnemyWaveInterval" => todo!(),
            "EnemyNormalWaveInterval" => todo!(),
            "EnemyNormalWaveDifficulty" => todo!(),
            "EnemyDiversity" => todo!(),
            "StationaryEnemyDiversity" => todo!(),
            "VeteranNormal" => todo!(),
            "VeteranLarge" => todo!(),
            "DisruptiveEnemyPoolCount" => todo!(),
            "MinPoolSize" => todo!(),
            "MaxActiveElites" => todo!(),
            "EnvironmentalDamageModifier" => todo!(),
            "PointExtractionScalar" => todo!(),
            "HazardBonus" => todo!(),
            "FriendlyFireModifier" => todo!(),
            "WaveStartDelayScale" => todo!(),
            "SpeedModifier" => todo!(),
            "AttackCooldownModifier" => todo!(),
            "ProjectileSpeedModifier" => todo!(),
            "HealthRegenerationMax" => todo!(),
            "ReviveHealthRatio" => todo!(),
            "EliteCooldown" => todo!(),
            "EnemyDescriptors" => todo!(),
            "EnemyPool" => todo!(),
            "CommonEnemies" => todo!(),
            "DisruptiveEnemies" => todo!(),
            "SpecialEnemies" => todo!(),
            "StationaryEnemies" => todo!(),
            "SeasonalEvents" => todo!(),
            "EscortMule" => todo!(),
            m => {
                handle_unknown_top_level_member(path, src, member_name, m)?;
            }
        }
    }

    Ok(())
}

type Diagnostics<'a> = Vec<Report<'a, (&'a String, std::ops::Range<usize>)>>;

fn handle_number_or_array<'d>(
    member_val: &Spanned<Json>,
    diagnostics: &mut Diagnostics<'d>,
    path: &'d String,
    src: &str,
    cd_member: &mut Spanned<ArrayOrSingleItem<usize>>,
) -> Result<(), anyhow::Error> {
    match &member_val.val {
        Json::Array(a) => {
            let mut arr = Vec::new();
            for val in &a.val {
                match &val.val {
                    Json::Num(n) => {
                        if (0.0..f64::MAX).contains(&n.val) {
                            arr.push(n.val as usize);
                        } else {
                            diagnostics.push(value_out_of_valid_range(
                                path,
                                n.val,
                                n.span,
                                0.0..f64::MAX,
                            ));
                        }
                    }
                    _ => {
                        unexpected_value_kind(path, member_val, "number")
                            .print((path, Source::from(src)))?;
                        bail!("unexpected JSON kind found in \"MaxActiveCritters\" value array");
                    }
                }
            }
            *cd_member = Spanned {
                span: member_val.span,
                val: ArrayOrSingleItem::Array(arr),
            };
        }
        Json::Num(n) => {
            if (0.0..f64::MAX).contains(&n.val) {
                *cd_member = Spanned {
                    span: n.span,
                    val: ArrayOrSingleItem::SingleItem(n.val as usize),
                };
            } else {
                diagnostics.push(value_out_of_valid_range(path, n.val, n.span, 0.0..f64::MAX));
            }
        }
        _ => {
            unexpected_value_kind(path, member_val, "number array or single number")
                .print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind found in \"{}\" member value",
                "MaxActiveCritters"
            );
        }
    };
    Ok(())
}

fn handle_unknown_top_level_member(
    path: &String,
    src: &str,
    member_name: &Spanned<String>,
    received_member_name: &str,
) -> Result<(), anyhow::Error> {
    const TOP_LEVEL_MEMBER_NAMES: [&'static str; 46] = [
        "Name",
        "Description",
        "MaxActiveCritters",
        "MaxActiveSwarmers",
        "MaxActiveEnemies",
        "ResupplyCost",
        "StartingNitra",
        "ExtraLargeEnemyDamageResistance",
        "ExtraLargeEnemyDamageResistanceB",
        "ExtraLargeEnemyDamageResistanceC",
        "ExtraLargeEnemyDamageResistanceD",
        "EnemyDamageResistance",
        "SmallEnemyDamageResistance",
        "EnemyDamageModifier",
        "EnemyCountModifier",
        "EncounterDifficulty",
        "StationaryDifficulty",
        "EnemyWaveInterval",
        "EnemyNormalWaveInterval",
        "EnemyNormalWaveDifficulty",
        "EnemyDiversity",
        "StationaryEnemyDiversity",
        "VeteranNormal",
        "VeteranLarge",
        "DisruptiveEnemyPoolCount",
        "MinPoolSize",
        "MaxActiveElites",
        "EnvironmentalDamageModifier",
        "PointExtractionScalar",
        "HazardBonus",
        "FriendlyFireModifier",
        "WaveStartDelayScale",
        "SpeedModifier",
        "AttackCooldownModifier",
        "ProjectileSpeedModifier",
        "HealthRegenerationMax",
        "ReviveHealthRatio",
        "EliteCooldown",
        "EnemyDescriptors",
        "EnemyPool",
        "CommonEnemies",
        "DisruptiveEnemies",
        "SpecialEnemies",
        "StationaryEnemies",
        "SeasonalEvents",
        "EscortMule",
    ];

    let mut report = Report::build(ReportKind::Error, path, member_name.span.start)
        .with_message(format!("unexpected member: \"{}\"", received_member_name))
        .with_label(Label::new((path, member_name.span.into_range())).with_color(Color::Red));
    if let Some(suggestion) = edit_distance::find_best_match_for_name(
        &TOP_LEVEL_MEMBER_NAMES,
        received_member_name,
        Some(3),
    ) {
        report.set_help(format!(
            "did you mean {} instead?",
            suggestion.fg(Color::Blue)
        ));
    }
    report.finish().print((path, Source::from(src)))?;
    bail!("unexpected top-level member");
}

fn unexpected_value_kind<'a, 'b>(
    path: &'a String,
    member_val: &'b Spanned<Json>,
    expected_kind: &'static str,
) -> Report<'a, (&'a String, std::ops::Range<usize>)> {
    Report::<(&'a String, std::ops::Range<usize>)>::build(
        ReportKind::Error,
        path,
        member_val.span.start,
    )
    .with_message(format!(
        "unexpected member value JSON kind: expected {} but found {}",
        expected_kind.fg(Color::Blue),
        member_val.val.kind_desc().fg(Color::Blue)
    ))
    .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
    .finish()
}

fn value_out_of_valid_range<'a, 'b>(
    path: &'a String,
    val: f64,
    span: SimpleSpan<usize>,
    valid_range: std::ops::Range<f64>,
) -> Report<'a, (&'a String, std::ops::Range<usize>)> {
    Report::<(&'a String, std::ops::Range<usize>)>::build(ReportKind::Error, path, span.start)
        .with_message(format!(
            "value {} outside of valid range {}",
            val.fg(Color::Blue),
            format!("[{}, {:+e})", valid_range.start, valid_range.end).fg(Color::Blue)
        ))
        .with_label(Label::new((path, span.into_range())).with_color(Color::Red))
        .finish()
}
