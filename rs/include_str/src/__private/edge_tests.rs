extern crate std;
use super::*;
use std::{format, fs, path::Path, string::String, vec::Vec};

// Capacity deliberately exceeds the largest accepted corpus fixture. The
// production macros additionally exercise exact-sized arrays in compiler tests.
fn parse(input: &str, jsonc: bool) -> String {
    let (bytes, len) = if jsonc {
        json::scan_jsonc::<16384>(input, true)
    } else {
        json::scan_json::<16384>(input, true)
    };
    let expected_len = if jsonc {
        jsonc_len(input)
    } else {
        json_len(input)
    };
    assert_eq!(len, expected_len);
    String::from(as_str(&bytes[..len]))
}

fn rejects(input: &str, jsonc: bool) {
    for emit in [false, true] {
        let result = std::panic::catch_unwind(|| {
            if jsonc {
                json::scan_jsonc::<16384>(input, emit)
            } else {
                json::scan_json::<16384>(input, emit)
            }
        });
        let panic = result.expect_err(input);
        let message = panic
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
            .unwrap();
        assert!(
            message.starts_with("include_str_json"),
            "{input:?}: {message}"
        );
    }
}

#[test]
fn independent_json_test_suite_corpus() {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/json-test-suite/test_parsing");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 318, "do not silently lose corpus cases");
    for path in paths {
        let name = path.file_name().unwrap().to_str().unwrap();
        let bytes = fs::read(&path).unwrap();
        let Ok(input) = core::str::from_utf8(&bytes) else {
            assert!(!name.starts_with("y_"), "{name}");
            // The compiler suite checks these bytes through include_str! too.
            continue;
        };
        let reject = name.starts_with("n_")
            || matches!(
                name,
                "i_structure_500_nested_arrays.json"
                    | "i_structure_UTF-8_BOM_empty_object.json"
                    | "i_string_utf16BE_no_BOM.json"
                    | "i_string_utf16LE_no_BOM.json"
            );
        if reject {
            let result = std::panic::catch_unwind(|| json_len(input));
            assert!(result.is_err(), "accepted {name}");
            // JSONC intentionally accepts these comment/trailing-comma fixtures.
            if matches!(
                name,
                "n_array_extra_comma.json"
                    | "n_array_number_and_comma.json"
                    | "n_object_trailing_comma.json"
                    | "n_object_trailing_comment.json"
                    | "n_object_trailing_comment_slash_open.json"
                    | "n_structure_object_with_comment.json"
            ) {
                let converted = parse(input, true);
                assert_eq!(parse(&converted, false), converted, "{name}");
            } else {
                assert!(
                    std::panic::catch_unwind(|| jsonc_len(input)).is_err(),
                    "JSONC accepted {name}"
                );
            }
        } else {
            let result = std::panic::catch_unwind(|| parse(input, false));
            let output = result.unwrap_or_else(|_| panic!("rejected {name}"));
            assert_eq!(parse(input, true), output, "{name}");
            assert_eq!(parse(&output, false), output, "{name}");
        }
    }
}

#[test]
fn every_ascii_escape_is_classified_in_keys_and_values() {
    for byte in 0..128u8 {
        if byte == b'u' {
            continue;
        } // Four-digit escapes tested separately.
        let token = format!("\"\\{}\"", byte as char);
        let valid = matches!(byte, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't');
        for input in [
            token.clone(),
            format!("[{token}]"),
            format!("{{{token}:0}}"),
        ] {
            for jsonc in [false, true] {
                if valid {
                    assert_eq!(parse(&input, jsonc), input);
                } else {
                    rejects(&input, jsonc);
                }
            }
        }
    }
}

#[test]
fn every_utf16_escape_is_preserved_without_decoding() {
    // Includes all surrogate halves and Unicode noncharacters, by documented policy.
    for unit in 0..=0xffffu32 {
        let input = format!("\"\\u{unit:04x}\"");
        assert_eq!(parse(&input, false), input);
        assert_eq!(parse(&input, true), input);
    }
}

#[test]
fn unicode_escape_digits_and_truncations_are_checked_at_every_position() {
    for position in 0..4 {
        for byte in 0..128u8 {
            let mut digits = [b'0'; 4];
            digits[position] = byte;
            let input = format!("\"\\u{}\"", core::str::from_utf8(&digits).unwrap());
            for jsonc in [false, true] {
                if byte.is_ascii_hexdigit() {
                    assert_eq!(parse(&input, jsonc), input);
                } else {
                    rejects(&input, jsonc);
                }
            }
        }
    }
    for input in [
        "\"\\",
        "\"\\u",
        "\"\\u0",
        "\"\\u00",
        "\"\\u000",
        "\"\\u0000",
    ] {
        rejects(input, false);
        rejects(input, true);
    }
}

#[test]
fn raw_controls_and_whitespace_are_context_sensitive() {
    for byte in 0..=127u8 {
        let c = byte as char;
        // Exclude quotes/backslashes here; they have their own exhaustive tests.
        if !matches!(byte, b'"' | b'\\') {
            let token = format!("\"{c}\"");
            for jsonc in [false, true] {
                if byte >= 32 {
                    assert_eq!(parse(&token, jsonc), token);
                } else {
                    rejects(&token, jsonc);
                }
            }
        }
        for input in [format!("{c}null"), format!("null{c}"), format!("[1,{c}2]")] {
            for jsonc in [false, true] {
                if matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                    let expected = if input.starts_with('[') {
                        "[1,2]"
                    } else {
                        "null"
                    };
                    assert_eq!(parse(&input, jsonc), expected);
                } else {
                    // Digits after a comma are legal parts of the next number.
                    if input.starts_with('[') && matches!(byte, b'1'..=b'9' | b'-') {
                        continue;
                    }
                    rejects(&input, jsonc);
                }
            }
        }
    }
}

#[test]
fn incomplete_documents_fail_at_every_truncation_boundary() {
    for complete in [
        r#"{"key":[true,false,null,-12.34e+5,"é🦀\u1234\""]}"#,
        r#"[{"a":{"b":[[],{}]}}]"#,
    ] {
        for (end, _) in complete.char_indices().skip(1) {
            rejects(&complete[..end], false);
            rejects(&complete[..end], true);
        }
        assert_eq!(parse(complete, false), complete);
    }
    let complete = "/*lead*/ {\"x\":[1,/*value*/2,],} /*tail*/";
    // Prefixes before the root object closes cannot be complete documents.
    let closing = complete.find("} ").unwrap();
    for end in 0..=closing {
        rejects(&complete[..end], true);
    }
}

#[test]
fn tokens_cannot_be_joined_by_whitespace_or_comments() {
    for token in ["true", "false", "null", "-12.34e+56", "0.01E-2"] {
        for split in 1..token.len() {
            for separator in [" ", "\t", "\r\n", "/**/", "//comment\n"] {
                let broken = format!("{}{}{}", &token[..split], separator, &token[split..]);
                for input in [
                    broken.clone(),
                    format!("[{broken}]"),
                    format!("{{\"a\":{broken}}}"),
                ] {
                    rejects(&input, false);
                    rejects(&input, true);
                }
            }
        }
    }
}

#[test]
fn root_values_cannot_be_concatenated() {
    let tokens = ["null", "true", "false", "1", "\"x\"", "[]", "{}"];
    for left in tokens {
        for right in tokens {
            for separator in [" ", "/**/", "//x\n"] {
                let input = format!("{left}{separator}{right}");
                rejects(&input, false);
                rejects(&input, true);
            }
        }
    }
}

#[test]
fn trailing_commas_depend_on_container_and_element_state() {
    for (input, expected) in [
        ("[0,]", "[0]"),
        ("{\"a\":0,}", "{\"a\":0}"),
        ("[{\"a\":[0,],},]", "[{\"a\":[0]}]"),
        ("[[],{},]", "[[],{}]"),
    ] {
        rejects(input, false);
        assert_eq!(parse(input, true), expected);
    }
    for input in [
        "[,]",
        "{,}",
        "[,,]",
        "[1,,2]",
        "[1,,]",
        "{\"a\":,}",
        "{\"a\",}",
        "[1,}",
        "{\"a\":1,]",
        "1,",
        "[,0]",
        "{,\"a\":0}",
    ] {
        rejects(input, false);
        rejects(input, true);
    }
}

#[test]
fn comment_delimiters_and_line_ending_boundaries() {
    for comment in [
        "/**/",
        "/***/",
        "/****/",
        "/* // ' \" {} [] */",
        "/* /* not nested */",
        "//x\n",
        "//x\r",
        "//x\r\n",
        "/*\0🦀*/",
    ] {
        assert_eq!(parse(&format!("{comment}null{comment}"), true), "null");
        rejects(&format!("{comment}null"), false);
    }
    for comment in ["/", "/*", "/*/", "/**", "/*x*", "/*x/"] {
        rejects(&format!("null{comment}"), true);
    }
    for terminator in ['\u{85}', '\u{2028}', '\u{2029}', '\u{b}', '\u{c}'] {
        rejects(&format!("[1,//comment{terminator}2]"), true);
    }
    for input in ["null//", "null// /*", "null// \"", "null//\0🦀"] {
        assert_eq!(parse(input, true), "null");
    }
}

#[test]
fn every_grammar_boundary_accepts_jsonc_comments() {
    let tokens = [
        "{",
        "\"a\"",
        ":",
        "[",
        "0",
        ",",
        "\"//literal/*x*/\"",
        "]",
        ",",
        "\"b\"",
        ":",
        "{",
        "}",
        "}",
    ];
    let expected = tokens.concat();
    for comment in ["/**/", "/*[]{}:,\"*/", "//comment\n", "//comment\r\n"] {
        for gap in 0..=tokens.len() {
            let input = format!(
                "{}{}{}",
                tokens[..gap].concat(),
                comment,
                tokens[gap..].concat()
            );
            assert_eq!(parse(&input, true), expected);
            rejects(&input, false);
        }
        assert_eq!(parse(&tokens.join(comment), true), expected);
    }
}

#[test]
fn object_and_mixed_nesting_limits_are_exact() {
    for depth in [1, 2, 127, 128, 129] {
        for mixed in [false, true] {
            let mut input = String::from("0");
            for level in 0..depth {
                input = if mixed && level % 2 == 0 {
                    format!("[{input}]")
                } else {
                    format!("{{\"a\":{input}}}")
                };
            }
            for jsonc in [false, true] {
                if depth <= 128 {
                    assert_eq!(parse(&input, jsonc), input);
                } else {
                    rejects(&input, jsonc);
                }
            }
        }
    }
}

#[test]
fn sibling_containers_do_not_accumulate_depth() {
    let input = format!("[{}{{}}]", "{\"a\":[0]},".repeat(1000));
    assert_eq!(parse(&input, false), input);
    assert_eq!(parse(&input, true), input);
}

#[test]
fn numbers_reject_non_json_spellings_in_every_value_position() {
    for number in [
        "00",
        "-00",
        "0123",
        "+0",
        "--1",
        "1.",
        ".1",
        "-.1",
        "1.e2",
        "1e",
        "1E",
        "1e+",
        "1e-",
        "1ee2",
        "1e++2",
        "0x1",
        "-0x1",
        "0b10",
        "0o7",
        "1_000",
        "NaN",
        "-Infinity",
        "Infinity",
        "1f",
        "1d",
    ] {
        for input in [
            String::from(number),
            format!("[{number}]"),
            format!("{{\"x\":{number}}}"),
        ] {
            rejects(&input, false);
            rejects(&input, true);
        }
    }
}

#[test]
fn all_quote_escape_run_lengths_preserve_json_strings() {
    for count in 0..128 {
        let prefix = "\\".repeat(count * 2);
        for tail in [
            "",
            "\\\"",
            "\\/\\b\\f\\n\\r\\t",
            "\\u0000",
            "\\uD800\\uDC00",
            " // /* 🦀",
        ] {
            let token = format!("\"{prefix}{tail}\"");
            assert_eq!(parse(&token, false), token);
            assert_eq!(parse(&format!("{token}//outside"), true), token);
        }
        let incomplete = format!("\"{}\"", "\\".repeat(count * 2 + 1));
        rejects(&incomplete, false);
        rejects(&incomplete, true);
    }
}

#[test]
fn exhaustive_short_text_transformations_match_standard_library() {
    let alphabet = ["a", "é", "🦀", " ", "\t", "\r", "\n"];
    for size in 0..=5 {
        for mut combination in 0..alphabet.len().pow(size) {
            let mut input = String::new();
            for _ in 0..size {
                input.push_str(alphabet[combination % alphabet.len()]);
                combination /= alphabet.len();
            }
            let (start, end) = trim_bounds(&input);
            assert_eq!(&input[start..end], input.trim());
            assert_eq!(trim_len(&input), input.trim().len());
            let expected_lines: String = input
                .split_inclusive('\n')
                .map(|line| {
                    let (text, ending) = if let Some(text) = line.strip_suffix("\r\n") {
                        (text, "\r\n")
                    } else if let Some(text) = line.strip_suffix('\n') {
                        (text, "\n")
                    } else {
                        (line, "")
                    };
                    format!("{}{ending}", text.trim())
                })
                .collect();
            let (bytes, len) = scan_trim_lines::<64>(&input, true);
            assert_eq!(len, trim_lines_len(&input));
            assert_eq!(as_str(&bytes[..len]), expected_lines, "{input:?}");
            for prefix in ["", "a", "aa", "é", "🦀", " ", "\t", " é"] {
                let expected: String = input
                    .split_inclusive('\n')
                    .map(|line| line.strip_prefix(prefix).unwrap_or(line))
                    .collect();
                let (bytes, len) = transforms::scan_strip_prefix::<64>(&input, prefix, true);
                assert_eq!(len, strip_prefix_len(&input, prefix));
                assert_eq!(as_str(&bytes[..len]), expected, "{input:?}, {prefix:?}");
            }
            for from in ["", "a", "aa", "é", "🦀", "\r\n", " "] {
                for to in ["", "🦀", "aa"] {
                    let expected = input.replace(from, to);
                    let (bytes, len) = transforms::scan_replace::<64>(&input, from, to, true);
                    assert_eq!(len, replace_len(&input, from, to));
                    assert_eq!(as_str(&bytes[..len]), expected, "{input:?}/{from:?}/{to:?}");
                }
            }
        }
    }
}

#[test]
fn unicode_scalars_are_never_split_by_text_transformations() {
    use std::string::ToString;
    for code in 0..=0x10ffff {
        let Some(c) = char::from_u32(code) else {
            continue;
        };
        let input = format!("{c}x{c}");
        let pattern = c.to_string();
        let (bytes, len) = transforms::scan_replace::<32>(&input, &pattern, "🦀", true);
        assert_eq!(as_str(&bytes[..len]), input.replace(&pattern, "🦀"));
        if c != '\r' && c != '\n' {
            let (bytes, len) = transforms::scan_strip_prefix::<32>(&input, &pattern, true);
            assert_eq!(as_str(&bytes[..len]), input.strip_prefix(c).unwrap());
        }
    }
}

#[test]
fn sql_all_dollar_tag_boundaries_and_case_sensitive_matches() {
    for tag in ["", "_", "a", "A", "a0", "é", "日本語"] {
        let delimiter = format!("{}{}{}", "$", tag, "$");
        let token = format!("{delimiter} -- /* \n  ' \" {delimiter}");
        for before in ["", " ", "\t", "\u{3000}", "/**/", "(", ","] {
            let input = format!("{before}{token}/*after*/;");
            let prefix = if matches!(before, "(" | ",") {
                before
            } else {
                ""
            };
            let expected = format!("{prefix}{token} ;");
            let (bytes, len) = scan::<1024>(&input, true);
            assert_eq!(len, sql_len(&input));
            assert_eq!(as_str(&bytes[..len]), expected);
        }
    }
    for input in ["$tag$body$TAG$", "$_$body$$", "$a0$body$a$"] {
        assert!(std::panic::catch_unwind(|| sql_len(input)).is_err());
    }
}

#[test]
fn sql_every_unterminated_quoted_prefix_is_rejected() {
    for token in [
        "'é  -- 🦀'",
        "\"é /* 🦀 */\"",
        "[é -- 🦀]",
        "$tag$é -- 🦀$tag$",
        "E'é\\' -- 🦀'",
    ] {
        let opener = if token.starts_with("$tag$") {
            5
        } else if token.starts_with('E') {
            2
        } else {
            1
        };
        for (end, _) in token.char_indices() {
            if end < opener {
                continue;
            }
            let input = &token[..end];
            assert!(
                std::panic::catch_unwind(|| sql_len(input)).is_err(),
                "{input:?}"
            );
        }
    }
}

#[test]
fn sql_whitespace_and_comment_markers_inside_all_quotes_remain_literal() {
    let whitespace: Vec<_> = (0..=0x10ffff)
        .filter_map(char::from_u32)
        .filter(|c| c.is_whitespace())
        .collect();
    for c in whitespace {
        for (open, close) in [("'", "'"), ("\"", "\""), ("[", "]"), ("$tag$", "$tag$")] {
            let token = format!("{open}{c}--/*x*/{c}{close}");
            let input = format!("{c}{token}{c}/*outside*/{c}");
            let (bytes, len) = scan::<256>(&input, true);
            assert_eq!(as_str(&bytes[..len]), token);
        }
    }
}
