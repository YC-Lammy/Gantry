use std::collections::HashMap;

use itertools::Itertools;
use pest::Parser;
use pest::error::Error;
use pest::iterators::Pair;
use pest_derive::Parser;

use super::cfg::{Config, Section, Value};

#[derive(Parser)]
#[grammar = "config/cfg.pest"]
struct CfgParser;

pub(super) fn parse_cfg(file: &str) -> Result<Config, Error<Rule>> {
    let mut pairs = CfgParser::parse(Rule::CONFIG, file)?;

    let cfg = pairs.next().unwrap();

    let mut sections = Vec::new();

    for p in cfg.into_inner() {
        if p.as_rule() == Rule::EOI {
            break;
        }

        debug_assert_eq!(p.as_rule(), Rule::SECTION);

        sections.push(parse_section(p));
    }

    return Ok(Config { sections });
}

fn parse_section(pair: Pair<Rule>) -> Section {
    let mut inner = pair.into_inner();

    let prefix_name_pair = inner.next().unwrap();

    debug_assert_eq!(prefix_name_pair.as_rule(), Rule::IDENT);

    let prefix_name = prefix_name_pair.as_str().to_string();

    let mut suffix_name = None;

    let mut values = HashMap::new();

    for pair in inner {
        match pair.as_rule() {
            Rule::IDENT => {
                debug_assert_eq!(suffix_name, None);

                suffix_name = Some(pair.as_str().to_string());
            }
            Rule::KEY_VALUE => {
                let (key, value) = parse_key_value(pair);
                values.insert(key, value);
            }
            _ => unreachable!(),
        }
    }

    return Section {
        prefix_name,
        suffix_name,
        values,
    };
}

fn parse_key_value(pair: Pair<Rule>) -> (String, Value) {
    let mut inner = pair.into_inner();

    let id_pair = inner.next().unwrap();

    debug_assert_eq!(id_pair.as_rule(), Rule::IDENT);

    let key = id_pair.as_str().to_string();

    let value_pair = inner.next().unwrap();

    debug_assert_eq!(value_pair.as_rule(), Rule::VALUE);

    let value = parse_value(value_pair.into_inner().next().unwrap());

    return (key, value);
}

fn parse_value(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::Number => Value::Number(fast_float::parse(pair.as_str()).unwrap()),
        Rule::Number_array | Rule::Multiline_number_array => {
            let array = pair
                .into_inner()
                .map(|p| {
                    debug_assert_eq!(p.as_rule(), Rule::Number);

                    fast_float::parse::<f64, _>(p.as_str()).unwrap()
                })
                .collect();

            Value::NumberArray(array)
        }
        Rule::Ratio => {
            let mut i = 1.0;

            for r in pair.as_str().split(',') {
                let (a, b) = r.split_once(':').unwrap();
                let a: f64 = fast_float::parse(a.trim()).unwrap();
                let b: f64 = fast_float::parse(b.trim()).unwrap();

                i = i * (a / b);
            }

            Value::Ratio(i)
        }
        Rule::Single_line_string => Value::String(pair.as_str().trim().to_string()),
        Rule::Multiline_string => {
            let s = pair
                .into_inner()
                .map(|p| {
                    debug_assert_eq!(p.as_rule(), Rule::Single_line_string);

                    p.as_str().trim()
                })
                .join("\n");

            Value::String(s)
        }
        Rule::String_array => {
            let s = pair
                .into_inner()
                .map(|p| {
                    debug_assert_eq!(p.as_rule(), Rule::String);

                    p.as_str().trim().to_string()
                })
                .collect();

            Value::StringArray(s)
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_cartesian_cfg() {
    const CARTESIAN_CFG: &str = include_str!("../../../config/example-cartesian.cfg");

    let re = parse_cfg(CARTESIAN_CFG);

    println!("{:#?}", re);
}

#[test]
fn test_kit_voron_cfg() {
    const KIT_VORON_CFG: &str = include_str!("../../../config/kit-voron2-250mm.cfg");

    let re = parse_cfg(KIT_VORON_CFG);

    println!("{:#?}", re);
}

#[test]
fn test_voron_trident_octopus_cfg() {
    const TRIDENT_CFG: &str = include_str!("../../../config/Trident-Octopus-Config.cfg");

    let re = parse_cfg(TRIDENT_CFG);

    println!("{:#?}", re);
}
