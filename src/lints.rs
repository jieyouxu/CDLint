use chumsky::prelude::*;

use crate::custom_difficulty::CustomDifficulty;

pub fn lint_empty_name<'a>(cd: &'a CustomDifficulty) -> Vec<Rich<'a, char>> {
    let mut errors = Vec::new();

    if cd.name.val.trim().is_empty() {
        errors.push(Rich::custom(
            cd.name.span,
            "the Custom Difficulty has an empty name",
        ));
    }

    return errors;
}
