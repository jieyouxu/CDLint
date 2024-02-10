#![feature(let_chains)]

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::{bail, Context};
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::container::Container;
use chumsky::prelude::*;
use clap::Parser as ClapParser;
use custom_difficulty::{ArrayOrSingleItem, EnemyDescriptor, EnemyPool, EscortMule, WeightedRange};
use serde::Deserialize;
use tracing::*;

use crate::custom_difficulty::{CustomDifficulty, PawnStats, Range};
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

fn dummy_sp() -> SimpleSpan {
    SimpleSpan::new(0, 0)
}

#[derive(Debug, Eq, Deserialize, Clone, Hash)]
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

type DiagnosticReport<'a> = Report<'a, (&'a String, std::ops::Range<usize>)>;
type Diagnostics<'a> = Vec<DiagnosticReport<'a>>;

pub enum ValidationResult<'d, T> {
    Ok(T),
    Err(DiagnosticReport<'d>),
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

fn handle_str<'d, 'a, 'n>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<String>,
    member_val: &Spanned<Json>,
    member_name: &'n str,
) -> anyhow::Result<()> {
    *target = if let Json::Str(s) = &member_val.val {
        s.to_owned()
    } else {
        unexpected_value_kind(path, member_val, "string").print((path, Source::from(src)))?;
        bail!("unexpected JSON kind found in \"{member_name}\" member value");
    };
    Ok(())
}

fn handle_single_item_or_array_number<'d, 'a, 'n, T: 'static>(
    diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    expected_ty: &'static str,
    target: &mut Spanned<ArrayOrSingleItem<T>>,
    member_val: &Spanned<Json>,
    member_name: &'n str,
    validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, T>,
) -> anyhow::Result<()> {
    use std::any::TypeId;

    match &member_val.val {
        Json::Array(a) if TypeId::of::<T>() == TypeId::of::<Vec<Json>>() => {
            let mut arr = Vec::new();
            for elem in &a.val {
                match validate(Box::new(elem.to_owned()), elem.span) {
                    ValidationResult::Ok(val) => {
                        arr.push(val);
                    }
                    ValidationResult::Err(report) => diag.push(report),
                }
            }
            *target = Spanned {
                span: member_val.span,
                val: ArrayOrSingleItem::Array(arr),
            };
        }
        Json::Num(n) if TypeId::of::<T>() == TypeId::of::<f64>() => {
            match validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => {
                    *target = Spanned {
                        span: member_val.span,
                        val: ArrayOrSingleItem::SingleItem(val),
                    }
                }
                ValidationResult::Err(report) => diag.push(report),
            }
        }
        Json::Str(s) if TypeId::of::<T>() == TypeId::of::<String>() => {
            match validate(Box::new(s.val.to_owned()), s.span) {
                ValidationResult::Ok(val) => {
                    *target = Spanned {
                        span: member_val.span,
                        val: ArrayOrSingleItem::SingleItem(val),
                    }
                }
                ValidationResult::Err(report) => diag.push(report),
            }
        }
        _ => {
            unexpected_value_kind(path, member_val, "{expected_ty} or array of {expected_ty}")
                .print((path, Source::from(src)))?;
            bail!("unexpected JSON kind {} found in \"{member_name}\" member value; expected {expected_ty} or array of {expected_ty}", member_val.val.kind_desc());
        }
    }
    Ok(())
}

fn handle_weighted_range_vec<'d, 'a, 'n, T>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<Vec<Spanned<WeightedRange<T>>>>,
    member_val: &Spanned<Json>,
    member_name: &'n str,
    weight_validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, f64>,
    range_validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, T>,
) -> anyhow::Result<()> {
    let mut arr = Vec::new();

    let Json::Array(a) = &member_val.val else {
        Report::build(ReportKind::Error, path, member_val.span.start)
            .with_message(format!(
                "expected \"{}\"'s value to be an array of weighted ranges",
                member_name.fg(Color::Blue)
            ))
            .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
            .finish()
            .print((path, Source::from(src)))?;
        bail!("expected an array of weighted ranges");
    };

    for elem in &a.val {
        let Json::Object(obj) = &elem.val else {
            Report::build(ReportKind::Error, path, elem.span.start)
                .with_message("expected a weighted range object")
                .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
                .finish()
                .print((path, Source::from(src)))?;
            bail!("expected a weighted range object");
        };

        let mut unique_members = BTreeMap::new();
        let mut unique_member_names = BTreeSet::new();
        for (member_name, member_val) in &obj.val {
            if !unique_member_names.insert(member_name.val.to_owned()) {
                unique_members.insert(member_name.to_owned(), member_val.to_owned());

                Report::build(ReportKind::Error, path, member_name.span.start)
                    .with_message(format!(
                        "member \"{}\" defined multiple times",
                        member_name.val.as_str().fg(Color::Blue)
                    ))
                    .with_label(
                        Label::new((
                            path,
                            unique_members
                                .iter()
                                .find(|(k, _)| k.val == member_name.val)
                                .unwrap()
                                .0
                                .span
                                .into_range(),
                        ))
                        .with_message(format!(
                            "member \"{}\" first defined here",
                            member_name.val.as_str().fg(Color::Blue)
                        ))
                        .with_color(Color::Red),
                    )
                    .with_label(
                        Label::new((path, member_name.span.into_range()))
                            .with_color(Color::Red)
                            .with_message(format!(
                                "member \"{}\" later redefined here",
                                member_name.val.as_str().fg(Color::Blue)
                            )),
                    )
                    .finish()
                    .print((path, Source::from(src)))?;
                bail!("duplicate member detected");
            }
        }

        const EXPECTED_MEMBERS: [&'static str; 2] = ["weight", "range"];

        for found_member_name in unique_members.keys() {
            if !EXPECTED_MEMBERS.contains(&found_member_name.val.as_str()) {
                let mut report =
                    Report::build(ReportKind::Error, path, found_member_name.span.start)
                        .with_message(format!(
                            "unexpected member \"{}\" when expecting a weighted range",
                            found_member_name.val
                        ))
                        .with_label(
                            Label::new((path, found_member_name.span.into_range()))
                                .with_color(Color::Red),
                        );
                if let Some(suggestion) = edit_distance::find_best_match_for_name(
                    &EXPECTED_MEMBERS,
                    &found_member_name.val,
                    Some(3),
                ) {
                    report.set_help(format!(
                        "did you mean {} instead?",
                        suggestion.fg(Color::Blue)
                    ));
                }
                report.finish().print((path, Source::from(src)))?;
                bail!("unexpected member when trying to process a weighted range object");
            }
        }

        let weight_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "weight")
            .map(|(_, v)| v)
            .unwrap();
        let weight = {
            let Json::Num(n) = &weight_member.val else {
                unexpected_value_kind(path, member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"weight\" member value; expected number",
                    member_val.val.kind_desc()
                );
            };
            match weight_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid weight value");
                }
            }
        };

        let range_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "range")
            .map(|(_, v)| v)
            .unwrap();
        let range = {
            let Json::Object(obj) = &range_member.val else {
                Report::build(ReportKind::Error, path, range_member.span.start)
                    .with_message("expected a range object")
                    .with_label(
                        Label::new((path, range_member.span.into_range())).with_color(Color::Red),
                    )
                    .finish()
                    .print((path, Source::from(src)))?;
                bail!("expected a range object");
            };

            let mut unique_members = BTreeMap::new();
            let mut unique_member_names = BTreeSet::new();

            for (member_name, member_val) in &obj.val {
                if !unique_member_names.insert(member_name.val.to_owned()) {
                    unique_members.insert(member_name.to_owned(), member_val.to_owned());

                    Report::build(ReportKind::Error, path, member_name.span.start)
                        .with_message(format!(
                            "member \"{}\" defined multiple times",
                            member_name.val.as_str().fg(Color::Blue)
                        ))
                        .with_label(
                            Label::new((
                                path,
                                unique_members
                                    .iter()
                                    .find(|(k, _)| k.val == member_name.val)
                                    .unwrap()
                                    .0
                                    .span
                                    .into_range(),
                            ))
                            .with_message(format!(
                                "member \"{}\" first defined here",
                                member_name.val.as_str().fg(Color::Blue)
                            ))
                            .with_color(Color::Red),
                        )
                        .with_label(
                            Label::new((path, member_name.span.into_range()))
                                .with_color(Color::Red)
                                .with_message(format!(
                                    "member \"{}\" later redefined here",
                                    member_name.val.as_str().fg(Color::Blue)
                                )),
                        )
                        .finish()
                        .print((path, Source::from(src)))?;
                    bail!("duplicate member detected");
                }
            }

            const EXPECTED_MEMBERS: [&'static str; 2] = ["min", "max"];

            for found_member_name in unique_members.keys() {
                if !EXPECTED_MEMBERS.contains(&found_member_name.val.as_str()) {
                    let mut report =
                        Report::build(ReportKind::Error, path, found_member_name.span.start)
                            .with_message(format!(
                                "unexpected member \"{}\" when expecting a range",
                                found_member_name.val
                            ))
                            .with_label(
                                Label::new((path, found_member_name.span.into_range()))
                                    .with_color(Color::Red),
                            );
                    if let Some(suggestion) = edit_distance::find_best_match_for_name(
                        &EXPECTED_MEMBERS,
                        &found_member_name.val,
                        Some(1),
                    ) {
                        report.set_help(format!(
                            "did you mean {} instead?",
                            suggestion.fg(Color::Blue)
                        ));
                    }
                    report.finish().print((path, Source::from(src)))?;
                    bail!("unexpected member when trying to process a range object");
                }
            }

            let min_member = unique_members
                .iter()
                .find(|(k, _)| k.val == "min")
                .map(|(_, v)| v)
                .unwrap();
            let min = {
                let Json::Num(n) = &min_member.val else {
                    unexpected_value_kind(path, member_val, "number")
                        .print((path, Source::from(src)))?;
                    bail!(
                        "unexpected JSON kind {} found in \"min\" member value; expected number",
                        member_val.val.kind_desc()
                    );
                };
                match range_validate(Box::new(n.val), n.span) {
                    ValidationResult::Ok(val) => val,
                    ValidationResult::Err(report) => {
                        report.print((path, Source::from(src)))?;
                        bail!("invalid min value");
                    }
                }
            };

            let max_member = unique_members
                .iter()
                .find(|(k, _)| k.val == "max")
                .map(|(_, v)| v)
                .unwrap();
            let max = {
                let Json::Num(n) = &max_member.val else {
                    unexpected_value_kind(path, member_val, "number")
                        .print((path, Source::from(src)))?;
                    bail!(
                        "unexpected JSON kind {} found in \"max\" member value; expected number",
                        member_val.val.kind_desc()
                    );
                };
                match range_validate(Box::new(n.val), n.span) {
                    ValidationResult::Ok(val) => val,
                    ValidationResult::Err(report) => {
                        report.print((path, Source::from(src)))?;
                        bail!("invalid max value");
                    }
                }
            };

            Range {
                min: Spanned {
                    span: min_member.span,
                    val: min,
                },
                max: Spanned {
                    span: max_member.span,
                    val: max,
                },
            }
        };

        arr.push(Spanned {
            span: elem.span,
            val: WeightedRange {
                weight: Spanned {
                    span: weight_member.span,
                    val: weight,
                },
                range: Spanned {
                    span: range_member.span,
                    val: range,
                },
            },
        });
    }

    *target = Spanned {
        span: member_val.span,
        val: arr,
    };

    Ok(())
}

fn handle_number<'d, 'a, 'n, T>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<T>,
    member_val: &Spanned<Json>,
    member_name: &'n str,
    validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, T>,
) -> anyhow::Result<()> {
    let Json::Num(n) = &member_val.val else {
        unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
        bail!(
            "unexpected JSON kind {} found in \"{member_name}\" member value; expected number",
            member_val.val.kind_desc()
        );
    };
    let val = match validate(Box::new(n.val), n.span) {
        ValidationResult::Ok(val) => val,
        ValidationResult::Err(report) => {
            report.print((path, Source::from(src)))?;
            bail!("invalid value");
        }
    };

    *target = Spanned {
        span: member_val.span,
        val,
    };

    Ok(())
}

fn handle_range<'d, 'a, 'n, T>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<Range<T>>,
    member_val: &Spanned<Json>,
    _member_name: &'n str,
    validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, T>,
) -> anyhow::Result<()> {
    let Json::Object(obj) = &member_val.val else {
        Report::build(ReportKind::Error, path, member_val.span.start)
            .with_message("expected a range object")
            .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
            .finish()
            .print((path, Source::from(src)))?;
        bail!("expected a range object");
    };

    let mut unique_members = BTreeMap::new();
    let mut unique_member_names = BTreeSet::new();
    for (member_name, member_val) in &obj.val {
        if !unique_member_names.insert(member_name.val.to_owned()) {
            unique_members.insert(member_name.to_owned(), member_val.to_owned());

            Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!(
                    "member \"{}\" defined multiple times",
                    member_name.val.as_str().fg(Color::Blue)
                ))
                .with_label(
                    Label::new((
                        path,
                        unique_members
                            .iter()
                            .find(|(k, _)| k.val == member_name.val)
                            .unwrap()
                            .0
                            .span
                            .into_range(),
                    ))
                    .with_message(format!(
                        "member \"{}\" first defined here",
                        member_name.val.as_str().fg(Color::Blue)
                    ))
                    .with_color(Color::Red),
                )
                .with_label(
                    Label::new((path, member_name.span.into_range()))
                        .with_color(Color::Red)
                        .with_message(format!(
                            "member \"{}\" later redefined here",
                            member_name.val.as_str().fg(Color::Blue)
                        )),
                )
                .finish()
                .print((path, Source::from(src)))?;
            bail!("duplicate member detected");
        }
    }

    const EXPECTED_MEMBERS: [&'static str; 2] = ["min", "max"];

    for found_member_name in unique_members.keys() {
        if !EXPECTED_MEMBERS.contains(&found_member_name.val.as_str()) {
            let mut report = Report::build(ReportKind::Error, path, found_member_name.span.start)
                .with_message(format!(
                    "unexpected member \"{}\" when expecting a range",
                    found_member_name.val
                ))
                .with_label(
                    Label::new((path, found_member_name.span.into_range())).with_color(Color::Red),
                );
            if let Some(suggestion) = edit_distance::find_best_match_for_name(
                &EXPECTED_MEMBERS,
                &found_member_name.val,
                Some(1),
            ) {
                report.set_help(format!(
                    "did you mean {} instead?",
                    suggestion.fg(Color::Blue)
                ));
            }
            report.finish().print((path, Source::from(src)))?;
            bail!("unexpected member when trying to process a range object");
        }
    }

    let min_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "min")
        .map(|(_, v)| v)
        .unwrap();
    let min = {
        let Json::Num(n) = &min_member.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"min\" member value; expected number",
                member_val.val.kind_desc()
            );
        };
        match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid min value");
            }
        }
    };

    let max_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "max")
        .map(|(_, v)| v)
        .unwrap();
    let max = {
        let Json::Num(n) = &max_member.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"max\" member value; expected number",
                member_val.val.kind_desc()
            );
        };
        match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid max value");
            }
        }
    };

    *target = Spanned {
        span: member_val.span,
        val: Range {
            min: Spanned {
                span: min_member.span,
                val: min,
            },
            max: Spanned {
                span: max_member.span,
                val: max,
            },
        },
    };

    Ok(())
}

fn handle_enemy_pool<'d, 'a, 'n>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<EnemyPool>,
    member_val: &Spanned<Json>,
    member_name: &'n str,
) -> anyhow::Result<()> {
    let Json::Object(obj) = &member_val.val else {
        Report::build(ReportKind::Error, path, member_val.span.start)
            .with_message("expected an enemy pool object")
            .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
            .finish()
            .print((path, Source::from(src)))?;
        bail!("expected a enemy pool object");
    };

    let mut unique_members = BTreeMap::new();
    let mut unique_member_names = BTreeSet::new();

    const EXPECTED_MEMBERS: [&'static str; 3] = ["clear", "add", "remove"];

    for (member_name, member_val) in &obj.val {
        if !unique_member_names.insert(member_name.val.to_owned()) {
            unique_members.insert(member_name.to_owned(), member_val.to_owned());
            Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!(
                    "member \"{}\" defined multiple times",
                    member_name.val.as_str().fg(Color::Blue)
                ))
                .with_label(
                    Label::new((
                        path,
                        unique_members
                            .iter()
                            .find(|(k, _)| k.val == member_name.val)
                            .unwrap()
                            .0
                            .span
                            .into_range(),
                    ))
                    .with_message(format!(
                        "member \"{}\" first defined here",
                        member_name.val.as_str().fg(Color::Blue)
                    ))
                    .with_color(Color::Red),
                )
                .with_label(
                    Label::new((path, member_name.span.into_range()))
                        .with_color(Color::Red)
                        .with_message(format!(
                            "member \"{}\" later redefined here",
                            member_name.val.as_str().fg(Color::Blue)
                        )),
                )
                .finish()
                .print((path, Source::from(src)))?;

            bail!("duplicate member detected");
        }

        if !EXPECTED_MEMBERS.contains(&member_name.val.as_str()) {
            let mut report = Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!("unexpected member: \"{}\"", member_name.val))
                .with_label(
                    Label::new((path, member_name.span.into_range())).with_color(Color::Red),
                );
            if let Some(suggestion) = edit_distance::find_best_match_for_name(
                &EXPECTED_MEMBERS,
                &member_name.val,
                Some(3),
            ) {
                report.set_help(format!(
                    "did you mean {} instead?",
                    suggestion.fg(Color::Blue)
                ));
            }
            report.finish().print((path, Source::from(src)))?;
            bail!("unexpected member");
        }
    }

    let clear_member = unique_members.iter().find(|(k, _)| k.val == "clear");
    let clear = if let Some((_, clear_member_val)) = clear_member {
        let Json::Bool(b) = &clear_member_val.val else {
            unexpected_value_kind(path, member_val, "bool").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"clear\" member value; expected bool",
                member_val.val.kind_desc()
            );
        };

        b.to_owned()
    } else {
        Spanned {
            span: dummy_sp(),
            val: false,
        }
    };

    let add_member = unique_members.iter().find(|(k, _)| k.val == "add");
    let add = if let Some((_, add_member_val)) = add_member {
        let Json::Array(a) = &add_member_val.val else {
            unexpected_value_kind(path, member_val, "array").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"add\" member value; expected array of strings",
                member_val.val.kind_desc()
            );
        };

        let mut descriptors = Vec::new();

        for elem in &a.val {
            let Json::Str(s) = &elem.val else {
                unexpected_value_kind(path, member_val, "string")
                    .print((path, Source::from(src)))?;
                bail!(
                    "found JSON kind {}, expected a string",
                    member_val.val.kind_desc()
                );
            };

            descriptors.push(s.to_owned());
        }

        Spanned {
            span: add_member_val.span,
            val: descriptors,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: Vec::new(),
        }
    };

    let remove_member = unique_members.iter().find(|(k, _)| k.val == "remove");
    let remove = if let Some((_, remove_member_val)) = remove_member {
        let Json::Array(a) = &remove_member_val.val else {
            unexpected_value_kind(path, member_val, "array").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"remove\" member value; expected array of strings",
                member_val.val.kind_desc()
            );
        };

        let mut pool = Vec::new();

        for elem in &a.val {
            let Json::Str(s) = &elem.val else {
                unexpected_value_kind(path, member_val, "string")
                    .print((path, Source::from(src)))?;
                bail!(
                    "found JSON kind {}, expected a string",
                    member_val.val.kind_desc()
                );
            };

            pool.push(s.to_owned());
        }

        Spanned {
            span: remove_member_val.span,
            val: pool,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: Vec::new(),
        }
    };

    *target = Spanned {
        span: member_val.span,
        val: EnemyPool { clear, add, remove },
    };

    Ok(())
}

fn handle_enemy_descriptors<'d, 'a, 'n>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<BTreeMap<Spanned<String>, Spanned<EnemyDescriptor>>>,
    member_val: &Spanned<Json>,
    _member_name: &'n str,
    f64_validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, f64>,
    usize_validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, usize>,
) -> anyhow::Result<()> {
    let Json::Object(obj) = &member_val.val else {
        Report::build(ReportKind::Error, path, member_val.span.start)
            .with_message("expected an enemy descriptors object")
            .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
            .finish()
            .print((path, Source::from(src)))?;
        bail!("expected a enemy descriptors object");
    };

    let mut descriptors = BTreeMap::new();

    let mut unique_members = BTreeMap::new();
    let mut unique_member_names = BTreeSet::new();
    for (name, ed) in &obj.val {
        if !unique_member_names.insert(name.val.to_owned()) {
            unique_members.insert(name.to_owned(), ed.to_owned());

            Report::build(ReportKind::Error, path, name.span.start)
                .with_message(format!(
                    "member \"{}\" defined multiple times",
                    name.val.as_str().fg(Color::Blue)
                ))
                .with_label(
                    Label::new((
                        path,
                        unique_members
                            .iter()
                            .find(|(k, _)| k.val == name.val)
                            .unwrap()
                            .0
                            .span
                            .into_range(),
                    ))
                    .with_message(format!(
                        "member \"{}\" first defined here",
                        name.val.as_str().fg(Color::Blue)
                    ))
                    .with_color(Color::Red),
                )
                .with_label(
                    Label::new((path, name.span.into_range()))
                        .with_color(Color::Red)
                        .with_message(format!(
                            "member \"{}\" later redefined here",
                            name.val.as_str().fg(Color::Blue)
                        )),
                )
                .finish()
                .print((path, Source::from(src)))?;

            bail!("duplicate member detected");
        }
    }

    for (name, ed) in &unique_members {
        let Json::Object(ed_obj) = &ed.val else {
            Report::build(ReportKind::Error, path, member_val.span.start)
                .with_message("expected an enemy descriptors object")
                .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
                .finish()
                .print((path, Source::from(src)))?;
            bail!("expected a enemy descriptor object");
        };

        const EXPECTED_MEMBERS: [&'static str; 14] = [
            "Base",
            "SpawnSpread",
            "IdealSpawnSize",
            "CanBeUsedForConstantPressure",
            "CanBeUsedInEncounters",
            "DifficultyRating",
            "MinSpawnCount",
            "MaxSpawnCount",
            "Rarity",
            "SpawnAmountModifier",
            "Elite",
            "Scale",
            "TimeDilation",
            "PawnStats",
        ];

        let mut unique_members = BTreeMap::new();
        let mut unique_member_names = BTreeSet::new();

        for (ed_member_name, ed_member_value) in &ed_obj.val {
            if !unique_member_names.insert(ed_member_name.val.to_owned()) {
                unique_members.insert(ed_member_name.to_owned(), ed_member_value.to_owned());

                Report::build(ReportKind::Error, path, name.span.start)
                    .with_message(format!(
                        "member \"{}\" defined multiple times",
                        name.val.as_str().fg(Color::Blue)
                    ))
                    .with_label(
                        Label::new((
                            path,
                            unique_members
                                .iter()
                                .find(|(k, _)| k.val == name.val)
                                .unwrap()
                                .0
                                .span
                                .into_range(),
                        ))
                        .with_message(format!(
                            "member \"{}\" first defined here",
                            name.val.as_str().fg(Color::Blue)
                        ))
                        .with_color(Color::Red),
                    )
                    .with_label(
                        Label::new((path, name.span.into_range()))
                            .with_color(Color::Red)
                            .with_message(format!(
                                "member \"{}\" later redefined here",
                                name.val.as_str().fg(Color::Blue)
                            )),
                    )
                    .finish()
                    .print((path, Source::from(src)))?;

                bail!("duplicate member detected");
            }

            if !EXPECTED_MEMBERS.contains(&ed_member_name.val.as_str()) {
                let mut report = Report::build(ReportKind::Error, path, ed_member_name.span.start)
                    .with_message(format!("unexpected member: \"{}\"", ed_member_name.val))
                    .with_label(
                        Label::new((path, ed_member_name.span.into_range())).with_color(Color::Red),
                    );
                if let Some(suggestion) = edit_distance::find_best_match_for_name(
                    &EXPECTED_MEMBERS,
                    &ed_member_name.val,
                    Some(3),
                ) {
                    report.set_help(format!(
                        "did you mean {} instead?",
                        suggestion.fg(Color::Blue)
                    ));
                }
                report.finish().print((path, Source::from(src)))?;
                bail!("unexpected member");
            }
        }

        let base_member = unique_members.iter().find(|(k, _)| k.val == "Base");
        let base = if let Some((_, base_member_val)) = base_member {
            let Json::Str(n) = &base_member_val.val else {
                unexpected_value_kind(path, member_val, "string")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"Base\" member value; expected string",
                    base_member_val.val.kind_desc()
                );
            };
            n.to_owned()
        } else {
            Spanned {
                span: dummy_sp(),
                val: String::new(),
            }
        };

        let spawn_spread_member = unique_members.iter().find(|(k, _)| k.val == "SpawnSpread");
        let spawn_spread = if let Some((_, spawn_spread_member_val)) = spawn_spread_member {
            let Json::Num(n) = &spawn_spread_member_val.val else {
                unexpected_value_kind(path, spawn_spread_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"SpawnSpread\" member value; expected number",
                spawn_spread_member_val.val.kind_desc()
            );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid SpawnSpread value");
                }
            };
            Spanned {
                span: spawn_spread_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let ideal_spawn_size_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "IdealSpawnSize");
        let ideal_spawn_size =
            if let Some((_, ideal_spawn_size_member_val)) = ideal_spawn_size_member {
                let Json::Num(n) = &ideal_spawn_size_member_val.val else {
                    unexpected_value_kind(path, ideal_spawn_size_member_val, "number")
                        .print((path, Source::from(src)))?;
                    bail!(
                "unexpected JSON kind {} found in \"IdealSpawnSize\" member value; expected number",
                ideal_spawn_size_member_val.val.kind_desc()
            );
                };
                let val = match usize_validate(Box::new(n.val), n.span) {
                    ValidationResult::Ok(val) => val,
                    ValidationResult::Err(report) => {
                        report.print((path, Source::from(src)))?;
                        bail!("invalid IdealSpawnSize value");
                    }
                };
                Spanned {
                    span: ideal_spawn_size_member_val.span,
                    val,
                }
            } else {
                Spanned {
                    span: dummy_sp(),
                    val: 0,
                }
            };

        let can_be_used_for_constant_pressure_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "CanBeUsedForConstantPressure");
        let can_be_used_for_constant_pressure = if let Some((
            _,
            can_be_used_for_constant_pressure_member_val,
        )) = can_be_used_for_constant_pressure_member
        {
            let Json::Bool(b) = &can_be_used_for_constant_pressure_member_val.val else {
                unexpected_value_kind(path, can_be_used_for_constant_pressure_member_val, "bool")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"CanBeUsedForConstantPressure\" member value; expected bool",
                can_be_used_for_constant_pressure_member_val.val.kind_desc()
            );
            };
            b.to_owned()
        } else {
            Spanned {
                span: dummy_sp(),
                val: false,
            }
        };

        let can_be_used_in_encounters_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "CanBeUsedInEncounters");
        let can_be_used_in_encounters = if let Some((_, can_be_used_in_encounters_member_val)) =
            can_be_used_in_encounters_member
        {
            let Json::Bool(b) = &can_be_used_in_encounters_member_val.val else {
                unexpected_value_kind(path, can_be_used_in_encounters_member_val, "bool")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"CanBeUsedInEncounters\" member value; expected bool",
                can_be_used_in_encounters_member_val.val.kind_desc()
                );
            };
            b.to_owned()
        } else {
            Spanned {
                span: dummy_sp(),
                val: false,
            }
        };

        let difficulty_rating_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "DifficultyRating");
        let difficulty_rating = if let Some((_, difficulty_rating_member_val)) =
            difficulty_rating_member
        {
            let Json::Num(n) = &difficulty_rating_member_val.val else {
                unexpected_value_kind(path, difficulty_rating_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"DifficultyRating\" member value; expected number",
                difficulty_rating_member_val.val.kind_desc()
            );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid DifficultyRating value");
                }
            };
            Spanned {
                span: difficulty_rating_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let min_spawn_count_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "MinSpawnCount");
        let min_spawn_count = if let Some((_, min_spawn_count_member_val)) = min_spawn_count_member
        {
            let Json::Num(n) = &min_spawn_count_member_val.val else {
                unexpected_value_kind(path, min_spawn_count_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"MinSpawnCount\" member value; expected number",
                min_spawn_count_member_val.val.kind_desc()
            );
            };
            let val = match usize_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid MinSpawnCount value");
                }
            };
            Spanned {
                span: min_spawn_count_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0,
            }
        };

        let max_spawn_count_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "MaxSpawnCount");
        let max_spawn_count = if let Some((_, max_spawn_count_member_val)) = max_spawn_count_member
        {
            let Json::Num(n) = &max_spawn_count_member_val.val else {
                unexpected_value_kind(path, max_spawn_count_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                "unexpected JSON kind {} found in \"MaxSpawnCount\" member value; expected number",
                max_spawn_count_member_val.val.kind_desc()
            );
            };
            let val = match usize_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid MaxSpawnCount value");
                }
            };
            Spanned {
                span: max_spawn_count_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0,
            }
        };

        let rarity_member = unique_members.iter().find(|(k, _)| k.val == "Rarity");
        let rarity = if let Some((_, rarity_member_val)) = rarity_member {
            let Json::Num(n) = &rarity_member_val.val else {
                unexpected_value_kind(path, rarity_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"Rarity\" member value; expected number",
                    rarity_member_val.val.kind_desc()
                );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid Rarity value");
                }
            };
            Spanned {
                span: rarity_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let spawn_amount_modifier_member = unique_members
            .iter()
            .find(|(k, _)| k.val == "SpawnAmountModifier");
        let spawn_amount_modifier = if let Some((_, spawn_amount_modifier_member_val)) =
            spawn_amount_modifier_member
        {
            let Json::Num(n) = &spawn_amount_modifier_member_val.val else {
                unexpected_value_kind(path, spawn_amount_modifier_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"SpawnAmountModifier\" member value; expected number",
                    spawn_amount_modifier_member_val.val.kind_desc()
                );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid SpawnAmountModifier value");
                }
            };
            Spanned {
                span: spawn_amount_modifier_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let elite_member = unique_members.iter().find(|(k, _)| k.val == "Elite");
        let elite = if let Some((_, elite_member_val)) = elite_member {
            let Json::Bool(b) = &elite_member_val.val else {
                unexpected_value_kind(path, elite_member_val, "bool")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"Elite\" member value; expected bool",
                    elite_member_val.val.kind_desc()
                );
            };
            b.to_owned()
        } else {
            Spanned {
                span: dummy_sp(),
                val: false,
            }
        };

        let scale_member = unique_members.iter().find(|(k, _)| k.val == "Scale");
        let scale = if let Some((_, scale_member_val)) = scale_member {
            let Json::Num(n) = &scale_member_val.val else {
                unexpected_value_kind(path, scale_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"Scale\" member value; expected number",
                    scale_member_val.val.kind_desc()
                );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid Scale value");
                }
            };
            Spanned {
                span: scale_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let time_dilation_member = unique_members.iter().find(|(k, _)| k.val == "TimeDilation");
        let time_dilation = if let Some((_, time_dilation_member_val)) = time_dilation_member {
            let Json::Num(n) = &time_dilation_member_val.val else {
                unexpected_value_kind(path, time_dilation_member_val, "number")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"TimeDilation\" member value; expected number",
                    time_dilation_member_val.val.kind_desc()
                );
            };
            let val = match f64_validate(Box::new(n.val), n.span) {
                ValidationResult::Ok(val) => val,
                ValidationResult::Err(report) => {
                    report.print((path, Source::from(src)))?;
                    bail!("invalid TimeDilation value");
                }
            };
            Spanned {
                span: time_dilation_member_val.span,
                val,
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: 0.0,
            }
        };

        let pawn_stats_member = unique_members.iter().find(|(k, _)| k.val == "PawnStats");
        let pawn_stats = if let Some((_, pawn_stats_member_val)) = pawn_stats_member {
            let Json::Object(pawn_stats_map) = &pawn_stats_member_val.val else {
                unexpected_value_kind(path, pawn_stats_member_val, "object")
                    .print((path, Source::from(src)))?;
                bail!(
                    "unexpected JSON kind {} found in \"PawnStats\" member value; expected object",
                    pawn_stats_member_val.val.kind_desc()
                );
            };

            let mut unique_members = BTreeMap::new();
            let mut unique_member_names = BTreeSet::new();
            for (member_name, member_val) in &pawn_stats_map.val {
                if !unique_member_names.insert(member_name.val.to_owned()) {
                    unique_members.insert(member_name.to_owned(), member_val.to_owned());

                    Report::build(ReportKind::Error, path, member_name.span.start)
                        .with_message(format!(
                            "member \"{}\" defined multiple times",
                            member_name.val.as_str().fg(Color::Blue)
                        ))
                        .with_label(
                            Label::new((
                                path,
                                unique_members
                                    .iter()
                                    .find(|(k, _)| k.val == member_name.val)
                                    .unwrap()
                                    .0
                                    .span
                                    .into_range(),
                            ))
                            .with_message(format!(
                                "member \"{}\" first defined here",
                                member_name.val.as_str().fg(Color::Blue)
                            ))
                            .with_color(Color::Red),
                        )
                        .with_label(
                            Label::new((path, member_name.span.into_range()))
                                .with_color(Color::Red)
                                .with_message(format!(
                                    "member \"{}\" later redefined here",
                                    member_name.val.as_str().fg(Color::Blue)
                                )),
                        )
                        .finish()
                        .print((path, Source::from(src)))?;
                    bail!("duplicate member detected");
                }
            }

            const EXPECTED_MEMBERS: [&'static str; 48] = [
                "PST_BarrelKicking",
                "PST_CarriableThrowing",
                "PST_CarryingCapacity",
                "PST_CarryingSpeedModifier",
                "PST_CaveLeechSense",
                "PST_ColdResistance",
                "PST_CorrosiveResistance",
                "PST_DamageBonus",
                "PST_DamageFromPlayers",
                "PST_DamageResistance",
                "PST_DepositSpeed",
                "PST_DirtMiningStrength",
                "PST_ElectricResistance",
                "PST_EventExplosionResistance",
                "PST_ExplodeOnDeath",
                "PST_ExplosionResistance",
                "PST_FallingResistance",
                "PST_FireResistance",
                "PST_FlareThrowStrength",
                "PST_FriendlyFire",
                "PST_GoldMining",
                "PST_HoverBootsDuration",
                "PST_InternalDamageResistance",
                "PST_KineticResistance",
                "PST_MaxHealth",
                "PST_MaxShield",
                "PST_MeleeDamage",
                "PST_MorkiteMining",
                "PST_MovementSpeed",
                "PST_MovementSpeedEnvironmentalPenalty",
                "PST_MovementSpeedEnvironmentalPenaltyReduction",
                "PST_MovementSpeedPenalty",
                "PST_MovementSpeedPenaltyReduction",
                "PST_PhysicalResistance",
                "PST_PoisonResistance",
                "PST_PowerAttackCooldownRate",
                "PST_RadiationResistance",
                "PST_RedSugarHeal",
                "PST_ResourceMiningStrength",
                "PST_ResupplyHealing",
                "PST_ResupplySpeed",
                "PST_ReviveSpeed",
                "PST_RockMiningStrength",
                "PST_ShieldRegeneratoinRate",
                "PST_SlipperyFloor",
                "PST_SprintSpeed",
                "PST_Ziplline_DownBoost",
                "PST_ZipllineSpee",
            ];

            for found_member_name in unique_members.keys() {
                if !EXPECTED_MEMBERS.contains(&found_member_name.val.as_str()) {
                    let mut report =
                        Report::build(ReportKind::Error, path, found_member_name.span.start)
                            .with_message(format!(
                                "unexpected member \"{}\" when expecting a weighted range",
                                found_member_name.val
                            ))
                            .with_label(
                                Label::new((path, found_member_name.span.into_range()))
                                    .with_color(Color::Red),
                            );
                    if let Some(suggestion) = edit_distance::find_best_match_for_name(
                        &EXPECTED_MEMBERS,
                        &found_member_name.val,
                        Some(3),
                    ) {
                        report.set_help(format!(
                            "did you mean {} instead?",
                            suggestion.fg(Color::Blue)
                        ));
                    }
                    report.finish().print((path, Source::from(src)))?;
                    bail!("unexpected member when trying to process a weighted range object");
                }
            }

            let mut pawn_stats = BTreeMap::new();

            for (name, val) in &unique_members {
                let Json::Num(pawn_stat_val) = &val.val else {
                    unexpected_value_kind(path, val, "number").print((path, Source::from(src)))?;
                    bail!(
                        "unexpected JSON kind {} found in \"PawnStats\" member value; expected number",
                        val.val.kind_desc()
                    );
                };

                pawn_stats.insert(name.to_owned(), pawn_stat_val.to_owned());
            }

            Spanned {
                span: member_val.span,
                val: PawnStats(pawn_stats),
            }
        } else {
            Spanned {
                span: dummy_sp(),
                val: PawnStats::default(),
            }
        };

        descriptors.insert(
            name.to_owned(),
            Spanned {
                span: name.span,
                val: EnemyDescriptor {
                    base,
                    spawn_spread,
                    ideal_spawn_size,
                    can_be_used_for_constant_pressure,
                    can_be_used_in_encounters,
                    difficulty_rating,
                    min_spawn_count,
                    max_spawn_count,
                    rarity,
                    spawn_amount_modifier,
                    elite,
                    scale,
                    time_dilation,
                    pawn_stats,
                },
            },
        );
    }

    *target = Spanned {
        span: member_val.span,
        val: descriptors,
    };

    Ok(())
}

fn handle_escort_mule<'d, 'a, 'n>(
    _diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    target: &mut Spanned<EscortMule>,
    member_val: &Spanned<Json>,
    _member_name: &'n str,
    validate: impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'d, f64>,
) -> anyhow::Result<()> {
    let Json::Object(obj) = &member_val.val else {
        Report::build(ReportKind::Error, path, member_val.span.start)
            .with_message("expected an escort mule object")
            .with_label(Label::new((path, member_val.span.into_range())).with_color(Color::Red))
            .finish()
            .print((path, Source::from(src)))?;
        bail!("expected a escort mule object");
    };

    const EXPECTED_MEMBERS: [&'static str; 4] = [
        "FriendlyFireModifier",
        "NeutralDamageModifier",
        "BigHitDamageModifier",
        "BigHitDamageReductionThreshold",
    ];

    let mut unique_member_names = BTreeSet::new();
    let mut unique_members = BTreeMap::new();

    for (member_name, member_val) in &obj.val {
        if !unique_member_names.insert(member_name.val.to_owned()) {
            unique_members.insert(member_name.to_owned(), member_val.to_owned());

            Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!(
                    "member \"{}\" defined multiple times",
                    member_name.val.as_str().fg(Color::Blue)
                ))
                .with_label(
                    Label::new((
                        path,
                        unique_members
                            .iter()
                            .find(|(k, _)| k.val == member_name.val)
                            .unwrap()
                            .0
                            .span
                            .into_range(),
                    ))
                    .with_message(format!(
                        "member \"{}\" first defined here",
                        member_name.val.as_str().fg(Color::Blue)
                    ))
                    .with_color(Color::Red),
                )
                .with_label(
                    Label::new((path, member_name.span.into_range()))
                        .with_color(Color::Red)
                        .with_message(format!(
                            "member \"{}\" later redefined here",
                            member_name.val.as_str().fg(Color::Blue)
                        )),
                )
                .finish()
                .print((path, Source::from(src)))?;

            bail!("duplicate member detected");
        }

        if !EXPECTED_MEMBERS.contains(&member_name.val.as_str()) {
            let mut report = Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!("unexpected member: \"{}\"", member_name.val))
                .with_label(
                    Label::new((path, member_name.span.into_range())).with_color(Color::Red),
                );
            if let Some(suggestion) = edit_distance::find_best_match_for_name(
                &EXPECTED_MEMBERS,
                &member_name.val,
                Some(3),
            ) {
                report.set_help(format!(
                    "did you mean {} instead?",
                    suggestion.fg(Color::Blue)
                ));
            }
            report.finish().print((path, Source::from(src)))?;
            bail!("unexpected member");
        }
    }

    let friendly_fire_modifier_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "FriendlyFireModifier");
    let friendly_fire_modifier = if let Some((_, ffm_member_val)) = friendly_fire_modifier_member {
        let Json::Num(n) = &ffm_member_val.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"FriendlyFireModifier\" member value; expected number",
                ffm_member_val.val.kind_desc()
            );
        };
        let val = match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid FriendlyFireModifier value");
            }
        };
        Spanned {
            span: ffm_member_val.span,
            val,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: 0.0,
        }
    };

    let neutral_damage_modifier_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "NeutralDamageModifier");
    let neutral_damage_modifier = if let Some((_, ndm_member_val)) = neutral_damage_modifier_member
    {
        let Json::Num(n) = &ndm_member_val.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"NeutralDamageModifier\" member value; expected number",
                ndm_member_val.val.kind_desc()
            );
        };
        let val = match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid NeutralDamageModifier value");
            }
        };
        Spanned {
            span: ndm_member_val.span,
            val,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: 0.0,
        }
    };

    let big_hit_damage_modifier_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "BigHitDamageModifier");
    let big_hit_damage_modifier = if let Some((_, bhm_member_val)) = big_hit_damage_modifier_member
    {
        let Json::Num(n) = &bhm_member_val.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"BigHitDamageModifier\" member value; expected number",
                bhm_member_val.val.kind_desc()
            );
        };
        let val = match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid BigHitDamageModifier value");
            }
        };
        Spanned {
            span: bhm_member_val.span,
            val,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: 0.0,
        }
    };

    let big_hit_damage_reduction_threshold_member = unique_members
        .iter()
        .find(|(k, _)| k.val == "BigHitDamageReductionThreshold");
    let big_hit_damage_reduction_threshold = if let Some((_, bhm_member_val)) =
        big_hit_damage_reduction_threshold_member
    {
        let Json::Num(n) = &bhm_member_val.val else {
            unexpected_value_kind(path, member_val, "number").print((path, Source::from(src)))?;
            bail!(
                "unexpected JSON kind {} found in \"BigHitDamageReductionThreshold\" member value; expected number",
                bhm_member_val.val.kind_desc()
            );
        };
        let val = match validate(Box::new(n.val), n.span) {
            ValidationResult::Ok(val) => val,
            ValidationResult::Err(report) => {
                report.print((path, Source::from(src)))?;
                bail!("invalid BigHitDamageReductionThreshold value");
            }
        };
        Spanned {
            span: bhm_member_val.span,
            val,
        }
    } else {
        Spanned {
            span: dummy_sp(),
            val: 0.0,
        }
    };

    *target = Spanned {
        span: member_val.span,
        val: EscortMule {
            friendly_fire_modifier,
            neutral_damage_modifier,
            big_hit_damage_modifier,
            big_hit_damage_reduction_threshold,
        },
    };

    Ok(())
}

fn handle_top_level_members<'d, 'a>(
    diag: &mut Diagnostics<'d>,
    path: &'d String,
    src: &'a str,
    cd: &mut CustomDifficulty,
    top_level_members: &Vec<(Spanned<String>, Spanned<Json>)>,
) -> anyhow::Result<()> {
    let mut unique_top_level_members = BTreeMap::new();
    let mut unique_top_level_member_names = BTreeSet::new();
    for (member_name, member_val) in top_level_members {
        if !unique_top_level_member_names.insert(member_name.val.to_owned()) {
            unique_top_level_members.insert(member_name.to_owned(), member_val.to_owned());

            Report::build(ReportKind::Error, path, member_name.span.start)
                .with_message(format!(
                    "member \"{}\" defined multiple times",
                    member_name.val.as_str().fg(Color::Blue)
                ))
                .with_label(
                    Label::new((
                        path,
                        unique_top_level_members
                            .iter()
                            .find(|(k, _)| k.val == member_name.val)
                            .unwrap()
                            .0
                            .span
                            .into_range(),
                    ))
                    .with_message(format!(
                        "member \"{}\" first defined here",
                        member_name.val.as_str().fg(Color::Blue)
                    ))
                    .with_color(Color::Red),
                )
                .with_label(
                    Label::new((path, member_name.span.into_range()))
                        .with_color(Color::Red)
                        .with_message(format!(
                            "member \"{}\" later redefined here",
                            member_name.val.as_str().fg(Color::Blue)
                        )),
                )
                .finish()
                .print((path, Source::from(src)))?;

            bail!("duplicate top-level member detected");
        }
    }

    for (member_name, member_val) in unique_top_level_members {
        let found_member_name = member_name.val.as_str();
        match found_member_name {
            "Name" => handle_str(
                diag,
                path,
                src,
                &mut cd.name,
                &member_val,
                found_member_name,
            )?,
            "Description" => handle_str(
                diag,
                path,
                src,
                &mut cd.description,
                &member_val,
                found_member_name,
            )?,
            "MaxActiveCritters" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.max_active_critters,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "MaxActiveSwarmers" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.max_active_swarmers,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "MaxActiveEnemies" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.max_active_enemies,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "ResupplyCost" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.resupply_cost,
                &member_val,
                "ResupplyCost",
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "StartingNitra" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.starting_nitra,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "ExtraLargeEnemyDamageResistance" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.extra_large_enemy_damage_resistance,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "ExtraLargeEnemyDamageResistanceB" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.extra_large_enemy_damage_resistance_b,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "ExtraLargeEnemyDamageResistanceC" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.extra_large_enemy_damage_resistance_c,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "ExtraLargeEnemyDamageResistanceD" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.extra_large_enemy_damage_resistance_d,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "EnemyDamageResistance" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.enemy_damage_resistance,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "SmallEnemyDamageResistance" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.small_enemy_damage_resistance,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "EnemyDamageModifier" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.enemy_damage_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "EnemyCountModifier" => handle_single_item_or_array_number(
                diag,
                path,
                src,
                "number",
                &mut cd.enemy_count_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "EncounterDifficulty" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.encounter_difficulty,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "StationaryDifficulty" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.stationary_difficulty,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnemyWaveInterval" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.enemy_wave_interval,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnemyNormalWaveInterval" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.enemy_normal_wave_interval,
                &member_val,
                "EnemyNormalWaveInterval",
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnemyNormalWaveDifficulty" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.enemy_normal_wave_difficulty,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnemyDiversity" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.enemy_diversity,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "StationaryEnemyDiversity" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.stationary_enemy_diversity,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "VeteranNormal" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.veteran_normal,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "VeteranLarge" => handle_weighted_range_vec(
                diag,
                path,
                src,
                &mut cd.veteran_large,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "DisruptiveEnemyPoolCount" => handle_range(
                diag,
                path,
                src,
                &mut cd.disruptive_enemy_pool_count,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "MinPoolSize" => handle_number(
                diag,
                path,
                src,
                &mut cd.min_pool_size,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "MaxActiveElites" => handle_number(
                diag,
                path,
                src,
                &mut cd.max_active_elites,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnvironmentalDamageModifier" => handle_number(
                diag,
                path,
                src,
                &mut cd.environmental_damage_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "PointExtractionScalar" => handle_number(
                diag,
                path,
                src,
                &mut cd.point_extraction_scalar,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "HazardBonus" => handle_number(
                diag,
                path,
                src,
                &mut cd.hazard_bonus,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "FriendlyFireModifier" => handle_number(
                diag,
                path,
                src,
                &mut cd.friendly_fire_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "WaveStartDelayScale" => handle_number(
                diag,
                path,
                src,
                &mut cd.wave_start_delay_scale,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "SpeedModifier" => handle_number(
                diag,
                path,
                src,
                &mut cd.speed_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "AttackCooldownModifier" => handle_number(
                diag,
                path,
                src,
                &mut cd.attack_cooldown_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "ProjectileSpeedModifier" => handle_number(
                diag,
                path,
                src,
                &mut cd.projectile_speed_modifier,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "HealthRegenerationMax" => handle_number(
                diag,
                path,
                src,
                &mut cd.health_regeneration_max,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "ReviveHealthRatio" => handle_number(
                diag,
                path,
                src,
                &mut cd.revive_health_ratio,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "EliteCooldown" => handle_number(
                diag,
                path,
                src,
                &mut cd.elite_cooldown,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EnemyPool" => handle_enemy_pool(
                diag,
                path,
                src,
                &mut cd.enemy_pool,
                &member_val,
                found_member_name,
            )?,
            "CommonEnemies" => handle_enemy_pool(
                diag,
                path,
                src,
                &mut cd.common_enemies,
                &member_val,
                found_member_name,
            )?,
            "DisruptiveEnemies" => handle_enemy_pool(
                diag,
                path,
                src,
                &mut cd.disruptive_enemies,
                &member_val,
                found_member_name,
            )?,
            "SpecialEnemies" => handle_enemy_pool(
                diag,
                path,
                src,
                &mut cd.special_enemies,
                &member_val,
                found_member_name,
            )?,
            "StationaryEnemies" => handle_enemy_pool(
                diag,
                path,
                src,
                &mut cd.stationary_enemies,
                &member_val,
                found_member_name,
            )?,
            "EnemyDescriptors" => handle_enemy_descriptors(
                diag,
                path,
                src,
                &mut cd.enemy_descriptors,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
                mk_finite_nonnegative_f64_to_usize_validator(path),
            )?,
            "EscortMule" => handle_escort_mule(
                diag,
                path,
                src,
                &mut cd.escort_mule,
                &member_val,
                found_member_name,
                mk_finite_nonnegative_f64_validator(path),
            )?,
            "SeasonalEvents" => todo!(),
            m => {
                handle_unknown_top_level_member(path, src, &member_name, m)?;
            }
        }
    }

    Ok(())
}

fn mk_finite_nonnegative_f64_to_usize_validator<'a>(
    path: &'a String,
) -> impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'a, usize> {
    |val, span| {
        let val = val.downcast_ref::<f64>().unwrap();
        if !val.is_sign_negative() && val.is_finite() {
            ValidationResult::Ok(*val as u64 as usize)
        } else {
            ValidationResult::Err(mk_non_negative_and_finite_f64_report(path, span, *val))
        }
    }
}

fn mk_finite_nonnegative_f64_validator<'a>(
    path: &'a String,
) -> impl Fn(Box<dyn Any>, SimpleSpan) -> ValidationResult<'a, f64> {
    |val, span| {
        let val = val.downcast_ref::<f64>().unwrap();
        if !val.is_sign_negative() && val.is_finite() {
            ValidationResult::Ok(*val)
        } else {
            ValidationResult::Err(mk_non_negative_and_finite_f64_report(path, span, *val))
        }
    }
}

fn mk_non_negative_and_finite_f64_report(
    path: &String,
    span: SimpleSpan,
    val: f64,
) -> DiagnosticReport<'_> {
    Report::build(ReportKind::Error, path, span.start)
        .with_message(format!(
            "value {} must be non-negative and finite",
            val.fg(Color::Blue)
        ))
        .with_label(Label::new((path, span.into_range())).with_color(Color::Red))
        .finish()
}

fn handle_unknown_top_level_member(
    path: &String,
    src: &str,
    member_name: &Spanned<String>,
    received_member_name: &str,
) -> anyhow::Result<()> {
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
