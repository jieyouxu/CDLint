#![feature(let_chains)]
#![feature(min_specialization)]
#![feature(extract_if)]

use std::path::PathBuf;

use anyhow::{bail, Context};
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use confique::toml::FormatOptions;
use confique::Config as DeriveConfig;
use tracing::*;

use crate::config::Config;
use crate::custom_difficulty::CustomDifficulty;
use crate::parser::Json;
use crate::spanned::Spanned;

mod config;
mod custom_difficulty;
mod edit_distance;
mod handlers;
mod late_lints;
mod logging;
mod parser;
mod spanned;

#[derive(Debug, ClapParser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to a Custom Difficulty JSON file.
    input: PathBuf,
}

type DiagnosticReport<'a> = Report<'a, (&'a String, std::ops::Range<usize>)>;
type Diagnostics<'a> = Vec<DiagnosticReport<'a>>;

pub enum ValidationResult<'d, T> {
    Ok(T),
    Err(DiagnosticReport<'d>),
}

pub(crate) fn dummy_sp() -> SimpleSpan {
    SimpleSpan::new(0, 0)
}

fn main() -> anyhow::Result<()> {
    logging::setup_logging();

    let exe_path = std::env::current_exe()?;

    let config_path = exe_path.parent().unwrap().join("config.toml");
    info!("trying to read config from `{}`", config_path.display());
    debug!(?config_path);
    debug!("config exists: {}", config_path.exists());
    if !config_path.exists() {
        warn!("no existing config detected, default config will be generated and default config values will be used");
        info!("generating default config at `{}`", config_path.display());
        let default_config = confique::toml::template::<Config>(FormatOptions::default());
        std::fs::write(&config_path, default_config)?;
    }
    let config = Config::builder().file(&config_path).load()?;
    debug!(?config);

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
    debug!(?errors);

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

    handlers::handle_top_level_members(
        &mut diagnostics,
        &path,
        &json_string,
        &mut custom_difficulty,
        &top_level_members,
    )
    .context("trying to process top level members")?;

    late_lints::lint_empty_cd_name(&config, &custom_difficulty, &path, &mut diagnostics);
    late_lints::lint_undefined_enemy_descriptors(
        &config,
        &custom_difficulty,
        &path,
        &mut diagnostics,
    );
    late_lints::lint_min_larger_than_max(&config, &custom_difficulty, &path, &mut diagnostics);
    late_lints::lint_unused_custom_enemy_descriptors(
        &config,
        &custom_difficulty,
        &path,
        &mut diagnostics,
    );
    late_lints::lint_ambiguous_enemy_pool_add_remove(
        &config,
        &custom_difficulty,
        &path,
        &mut diagnostics,
    );
    late_lints::lint_cyclic_enemy_descriptor_references(
        &config,
        &custom_difficulty,
        &path,
        &mut diagnostics,
    )?;

    for diagnostic in &diagnostics {
        diagnostic.print((&path, Source::from(&json_string)))?;
    }

    Ok(())
}
