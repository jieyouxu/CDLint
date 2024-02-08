#![feature(let_chains)]

use std::path::PathBuf;

use anyhow::{bail, Context};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use serde::Deserialize;
use tracing::*;

use crate::custom_difficulty::CustomDifficulty;
use crate::edit_distance::edit_distance;
use crate::parser::Json;

mod custom_difficulty;
mod edit_distance;
mod lints;
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
            .eprint((&path, Source::from(&json_string)))
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

    // There are two kinds of lints:
    // 1. Early-pass lints: these lints are performed while parsing the CD JSON into the CD struct.
    // 2. Late-pass lints: these lints are performed on the built CD struct.

    for (member_name, member_val) in top_level_members {
        match member_name.val.as_str() {
            "Name" => todo!(),
            "Description" => todo!(),
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
                let mut report = Report::build(ReportKind::Error, &path, member_name.span.start)
                    .with_message(format!("unexpected member: \"{}\"", m))
                    .with_label(
                        Label::new((&path, member_name.span.into_range())).with_color(Color::Red),
                    );

                if let Some(suggestion) =
                    edit_distance::find_best_match_for_name(&TOP_LEVEL_MEMBER_NAMES, m, Some(3))
                {
                    report.set_help(format!(
                        "did you mean {} instead?",
                        suggestion.fg(Color::Blue)
                    ));
                }

                diagnostics.push(report.finish());
            }
        }
    }

    for diagnostic in diagnostics {
        diagnostic.print((&path, Source::from(&json_string)))?;
    }

    Ok(())
}
