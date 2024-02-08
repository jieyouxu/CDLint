use std::ops::Range;
use std::path::PathBuf;

use anyhow::{bail, Context};
use ariadne::{sources, Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use serde::Deserialize;
use tracing::*;

use crate::parser::Json;

mod custom_difficulty;
mod lints;
mod logging;
mod parser;

#[derive(Debug, ClapParser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to a Custom Difficulty JSON file.
    input: PathBuf,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidJson(Range<usize>),
    MissingCustomDifficultyName,
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

    Ok(())
}
