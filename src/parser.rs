//! Parser for a Custom Difficulty JSON.

use chumsky::prelude::*;

use crate::Spanned;

#[derive(Clone, Debug)]
pub enum Json {
    Invalid(Spanned<()>),
    Null(Spanned<()>),
    Bool(Spanned<bool>),
    Str(Spanned<String>),
    Num(Spanned<f64>),
    Array(Spanned<Vec<Spanned<Json>>>),
    Object(Spanned<Vec<(Spanned<String>, Spanned<Json>)>>),
}

impl Json {
    pub fn kind_desc(&self) -> &'static str {
        match self {
            Json::Invalid(_) => "invalid",
            Json::Null(_) => "null",
            Json::Bool(_) => "bool",
            Json::Str(_) => "string",
            Json::Num(_) => "number",
            Json::Array(_) => "array",
            Json::Object(_) => "object",
        }
    }
}

/// Taken from: <https://github.com/zesterer/chumsky/blob/main/examples/json.rs>.
pub fn parser<'a>() -> impl Parser<'a, &'a str, Spanned<Json>, extra::Err<Rich<'a, char>>> {
    recursive(|value| {
        let digits = text::digits(10).to_slice();

        let frac = just('.').then(digits);

        let exp = just('e')
            .or(just('E'))
            .then(one_of("+-").or_not())
            .then(digits)
            .labelled("exponent");

        let number = just('-')
            .or_not()
            .then(text::int(10))
            .then(frac.or_not())
            .then(exp.or_not())
            .to_slice()
            .map_with(|s: &str, e| {
                let n: f64 = s.parse().unwrap();
                Spanned {
                    span: e.span(),
                    val: n,
                }
            })
            .boxed()
            .labelled("number");

        let escape = just('\\')
            .then(choice((
                just('\\'),
                just('/'),
                just('"'),
                just('b').to('\x08'),
                just('f').to('\x0C'),
                just('n').to('\n'),
                just('r').to('\r'),
                just('t').to('\t'),
                just('u').ignore_then(text::digits(16).exactly(4).to_slice().validate(
                    |digits, e, emitter| {
                        char::from_u32(u32::from_str_radix(digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    },
                )),
            )))
            .ignored()
            .boxed()
            .labelled("escape character");

        let string = none_of("\\\"")
            .ignored()
            .or(escape)
            .repeated()
            .to_slice()
            .map(ToString::to_string)
            .delimited_by(just('"'), just('"'))
            .map_with(|val, e| Spanned {
                span: e.span(),
                val,
            })
            .boxed()
            .labelled("string");

        let array = value
            .clone()
            .separated_by(just(',').padded())
            .collect()
            .map_with(|val, e| Spanned {
                val,
                span: e.span(),
            })
            .padded()
            .delimited_by(just('['), just(']'))
            .boxed()
            .labelled("array");

        let member = string
            .clone()
            .then_ignore(just(':').padded())
            .then(value)
            .labelled("object member");
        let object = member
            .clone()
            .separated_by(just(',').padded())
            .collect()
            .map_with(|val, e| Spanned {
                val,
                span: e.span(),
            })
            .padded()
            .delimited_by(just('{'), just('}'))
            .boxed()
            .labelled("object");

        choice((
            just("null")
                .map_with(|_, e| Spanned {
                    span: e.span(),
                    val: Json::Null(Spanned {
                        span: e.span(),
                        val: (),
                    }),
                })
                .labelled("null"),
            just("true")
                .map_with(|_, e| Spanned {
                    span: e.span(),
                    val: Json::Bool(Spanned {
                        val: true,
                        span: e.span(),
                    }),
                })
                .labelled("true"),
            just("false")
                .map_with(|_, e| Spanned {
                    span: e.span(),
                    val: Json::Bool(Spanned {
                        val: false,
                        span: e.span(),
                    }),
                })
                .labelled("false"),
            number
                .map_with(|val, e| Spanned {
                    span: e.span(),
                    val: Json::Num(val),
                })
                .labelled("number"),
            string
                .map_with(|val, e| Spanned {
                    span: e.span(),
                    val: Json::Str(val),
                })
                .labelled("string"),
            array
                .map_with(|val, e| Spanned {
                    span: e.span(),
                    val: Json::Array(val),
                })
                .labelled("array"),
            object
                .map_with(|val, e| Spanned {
                    span: e.span(),
                    val: Json::Object(val),
                })
                .labelled("object"),
        ))
        .padded()
    })
    .labelled("Custom Difficulty JSON")
}
