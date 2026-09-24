extern crate std;
use super::*;
use std::{format, string::String};

fn replaced(input: &str, from: &str, to: &str) -> String {
    let (bytes, len) = transforms::scan_replace::<8192>(input, from, to, true);
    assert_eq!(len, replace_len(input, from, to));
    String::from(as_str(&bytes[..len]))
}

fn stripped(input: &str, prefix: &str) -> String {
    let (bytes, len) = transforms::scan_strip_prefix::<8192>(input, prefix, true);
    assert_eq!(len, strip_prefix_len(input, prefix));
    String::from(as_str(&bytes[..len]))
}

fn minified(input: &str) -> String {
    let (bytes, len) = json::scan_json::<8192>(input, true);
    assert_eq!(len, json_len(input));
    String::from(as_str(&bytes[..len]))
}

#[test]
fn replacement_matches_std_for_empty_overlapping_and_unicode_patterns() {
    for input in [
        "",
        "a",
        "aaaaa",
        "abababa",
        "café 🦀 café",
        "éé",
        "\0a\0",
        "a\r\nb",
        "  x  ",
    ] {
        for from in [
            "",
            "a",
            "aa",
            "aba",
            "é",
            "🦀",
            "\0",
            "\r\n",
            " ",
            "long absent pattern",
        ] {
            for to in ["", "a", "aa", "🦀", "é", "\n", "\0", "replacement"] {
                assert_eq!(
                    replaced(input, from, to),
                    input.replace(from, to),
                    "{input:?}/{from:?}/{to:?}"
                );
            }
        }
    }
}

#[test]
fn generated_replacements_match_standard_library() {
    let atoms = ["a", "b", "é", "🦀", "\r\n", "\0", " "];
    let mut seed = 78231_u64;
    for case in 0..2048 {
        let mut input = String::new();
        for _ in 0..case % 32 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            input.push_str(atoms[(seed >> 32) as usize % atoms.len()]);
        }
        let from = if case % 5 == 0 {
            String::new()
        } else {
            format!(
                "{}{}",
                atoms[case % atoms.len()],
                atoms[(case / 7) % atoms.len()]
            )
        };
        let to = atoms[(case / 3) % atoms.len()];
        assert_eq!(replaced(&input, &from, to), input.replace(&from, to));
    }
}

#[test]
fn prefix_stripping_preserves_line_endings_and_removes_only_one_match() {
    for (input, prefix, expected) in [
        ("", "", ""),
        ("", "x", ""),
        ("> ", "> ", ""),
        ("> a\n> b\r\nplain\n> ", "> ", "a\nb\r\nplain\n"),
        ("> > a", "> ", "> a"),
        (" > a", "> ", " > a"),
        ("éé\né🦀\r\né", "é", "é\n🦀\r\n"),
        ("x\rx\nx", "x", "\rx\n"),
        ("\n\r\n", "", "\n\r\n"),
        ("aaa", "aaaa", "aaa"),
    ] {
        assert_eq!(stripped(input, prefix), expected);
    }
    let lines = ["", "x", "xx", "> text", "  > text", "é🦀", "\0x"];
    for prefix in ["", "x", "> ", "  ", "é", "\0", "long prefix"] {
        for ending in ["\n", "\r\n"] {
            let input = format!("{}{ending}", lines.join(ending));
            let expected: String = input
                .split_inclusive('\n')
                .map(|line| line.strip_prefix(prefix).unwrap_or(line))
                .collect();
            assert_eq!(stripped(&input, prefix), expected);
        }
    }
}

#[test]
fn prefix_cannot_span_line_endings() {
    for prefix in ["\n", "\r", "x\n", "\r\n"] {
        assert!(std::panic::catch_unwind(|| strip_prefix_len("", prefix)).is_err());
    }
}

#[test]
fn json_preserves_literals_numeric_spelling_keys_and_escapes() {
    for input in [
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1.2300e+09",
        "1e999999",
        "123456789012345678901234567890",
        "[]",
        "{}",
        r#""""#,
        r#""a  b -- /* text */""#,
        r#""\"\\\/\b\f\n\r\t""#,
        r#""\u0000\uD834\uDD1E\uDEAD""#,
        r#""é 🦀 日本語""#,
        r#"{"x":1,"x":2,"a":3}"#,
        r#"[true,false,null,{"nested":[1,2]}]"#,
    ] {
        assert_eq!(minified(&format!(" \r\n\t{input}\r\n ")), input);
    }
    assert_eq!(
        minified(" { \"a\" : [ 1 , 2 ], \"b\" : { } } "),
        r#"{"a":[1,2],"b":{}}"#
    );
    assert_eq!(
        minified("\"\u{a0}\u{2028}\u{2029}\""),
        "\"\u{a0}\u{2028}\u{2029}\""
    );
}

#[test]
fn json_rejects_invalid_grammar_in_both_passes() {
    for input in [
        "",
        " ",
        "01",
        "-01",
        "-",
        "+1",
        ".1",
        "1.",
        "1e",
        "1e+",
        "1e-",
        "1 2",
        "1\n2",
        "true false",
        "tru",
        "True",
        "NaN",
        "Infinity",
        "[",
        "{",
        "[}",
        "{]",
        "[1,]",
        "[,1]",
        "[1,,2]",
        "[1 2]",
        r#"{"a":}"#,
        r#"{a:1}"#,
        r#"{"a" 1}"#,
        r#"{"a":1,}"#,
        r#"{"a":1 "b":2}"#,
        r#""unterminated"#,
        r#""\x""#,
        r#""\u""#,
        r#""\u123""#,
        r#""\uXXXX""#,
        "\"a\nb\"",
        "\"a\tb\"",
        "\"\0\"",
        "\"\\",
        "\"\\\"",
        "/*comment*/1",
        "//comment\n1",
        "null;",
        "\u{feff}{}",
        "\u{a0}null",
        "[1,\u{a0}2]",
        "[1,\u{b}2]",
        "[1,\u{c}2]",
    ] {
        for emit in [false, true] {
            let panic =
                std::panic::catch_unwind(|| json::scan_json::<8192>(input, emit)).expect_err(input);
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .unwrap();
            assert!(
                message.starts_with("include_str_json!:"),
                "{input:?}: {message}"
            );
        }
    }
    for control in 0..32u8 {
        let input = format!("\"{}\"", control as char);
        assert!(std::panic::catch_unwind(|| json_len(&input)).is_err());
    }
}

#[test]
fn json_nesting_limit_is_explicit_and_flat_arrays_are_unlimited() {
    for depth in [1, 2, 64, 128] {
        let input = format!("{} 0 {}", "[".repeat(depth), "]".repeat(depth));
        assert_eq!(
            minified(&input),
            format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
        );
    }
    let too_deep = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    assert!(std::panic::catch_unwind(|| json_len(&too_deep)).is_err());
    let input = format!("[{}0]", "0,".repeat(1024));
    assert_eq!(minified(&input), input);
}

#[test]
fn generated_json_preserves_known_tokens_and_is_idempotent() {
    let atoms = [
        "null",
        "true",
        "false",
        "-0",
        "1.230e+4",
        r#"" a  b ""#,
        r#""quote: \" slash: \\""#,
        r#""é 🦀 \u0041""#,
    ];
    for case in 0..2048 {
        let atom = atoms[case % atoms.len()];
        let mut pretty = String::from(atom);
        let mut expected = String::from(atom);
        for level in 0..case % 12 {
            if (case + level) % 2 == 0 {
                pretty = format!(" [ \r\n {pretty} , \t{atom} ] ");
                expected = format!("[{expected},{atom}]");
            } else {
                pretty = format!(" {{ \"key\" \t: {pretty} }} ");
                expected = format!("{{\"key\":{expected}}}");
            }
        }
        let actual = minified(&pretty);
        assert_eq!(actual, expected);
        assert_eq!(minified(&actual), actual);
    }
}

#[test]
fn all_number_components_are_validated_without_float_conversion() {
    for integer in ["0", "1", "123456789012345678901234567890"] {
        for sign in ["", "-"] {
            for fraction in ["", ".0", ".0012300"] {
                for exponent in ["", "e0", "e+99", "E-99", "e999999"] {
                    let number = format!("{sign}{integer}{fraction}{exponent}");
                    assert_eq!(minified(&number), number);
                }
            }
        }
    }
}
