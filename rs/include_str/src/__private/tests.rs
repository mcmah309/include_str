//! Expected outputs come from known tokens, not a second SQL scanner.
extern crate std;
use super::*;
use std::{format, string::String, vec::Vec};

fn compact(input: &str) -> String {
    let (bytes, len) = scan::<32768>(input, true);
    assert_eq!(len, sql_len(input), "sizing disagrees for {input:?}");
    assert!(len <= input.len(), "compaction grew the input");
    String::from(core::str::from_utf8(&bytes[..len]).expect("output must stay UTF-8"))
}

fn check(input: &str, expected: &str) {
    let actual = compact(input);
    assert_eq!(actual, expected, "input: {input:?}");
    assert_eq!(compact(&actual), actual, "compaction must be idempotent");
}

#[test]
fn empty_whitespace_and_comment_only_inputs() {
    for input in [
        "",
        " ",
        "\r\n\t",
        "--",
        "-- no newline",
        "-- 'unterminated",
        "-- /* not a block",
        "/**/",
        "/***/",
        "/****/",
        "/*/**/*/",
        "/* ' \" ` [ $$ */",
        "/* 🦀 日本語 */",
        " \t/* first */-- second\r\n/* third */ \n",
    ] {
        check(input, "");
    }
}

#[test]
fn comments_at_every_token_boundary() {
    let tokens = [
        "SELECT",
        "name",
        ",",
        "'a  -- b'",
        "FROM",
        "people",
        "WHERE",
        "id",
        "=",
        "$1",
        ";",
    ];
    let separators = [
        " ",
        "\t",
        "\n",
        "\r",
        "\r\n",
        "/**/",
        "/* x */",
        "/* a /* b */ c */",
        "-- x\n",
        "-- x\r",
        "-- x\r\n",
        " \t/**/ --x\n ",
    ];
    let expected = tokens.join(" ");
    for separator in separators {
        check(&tokens.join(separator), &expected);
        check(
            &format!("{separator}{}{separator}", tokens.join(separator)),
            &expected,
        );
    }
    // Removing comments must never manufacture operators or comment delimiters.
    for (input, expected) in [
        ("SELECT/**/name", "SELECT name"),
        ("1/**/2", "1 2"),
        ("-/**/-", "- -"),
        ("/**//**/", ""),
        ("/ /* x */ *", "/ *"),
        ("*/* x *//", "* /"),
        ("</**/=", "< ="),
        (">/**/>", "> >"),
        (":/**/:", ": :"),
        ("|/**/|", "| |"),
        ("'a'/**/'b'", "'a' 'b'"),
        ("E/**/'a'", "E 'a'"),
    ] {
        check(input, expected);
    }
}

#[test]
fn line_comments_stop_only_at_cr_or_lf() {
    for ending in ["\n", "\r", "\r\n", "\n\r"] {
        check(&format!("SELECT-- ' \" /* $$ 🦀{ending}1"), "SELECT 1");
    }
    for non_ending in ['\t', '\u{85}', '\u{2028}', '\u{2029}'] {
        check(
            &format!("SELECT-- ignored{non_ending}still ignored\n1"),
            "SELECT 1",
        );
    }
    check("SELECT 1;-- no final newline", "SELECT 1;");
    check("SELECT 1;-- /* unmatched\nSELECT 2;", "SELECT 1; SELECT 2;");
}

#[test]
fn nested_comments_ignore_quotes_and_line_comment_markers() {
    check("a/* ' \" ` [ -- $$ /* inner */ -- tail */b", "a b");
    for depth in [1, 2, 3, 16, 128, 1024] {
        check(
            &format!("a{}payload{}b", "/*".repeat(depth), "*/".repeat(depth)),
            "a b",
        );
    }
    check("a/* / * / ** *** / */b", "a b");
}

#[test]
fn all_quote_styles_preserve_every_payload_byte() {
    let payloads = [
        "",
        " ",
        "  \t\r\n ",
        "--",
        "/*",
        "*/",
        "/* nested /* x */ */",
        "a-- b\nc",
        "' \" ` [ ] $$ $tag$",
        "é 日本語 🦀",
        "\u{85}\u{a0}\u{2003}\u{2028}\u{3000}",
        "\\",
        "\\\\",
        "https://example.test/a?x=1--2",
        "\0",
        "\u{200b}\u{feff}",
    ];
    for (open, close) in [('\'', '\''), ('"', '"'), ('`', '`'), ('[', ']')] {
        for payload in payloads {
            let escaped = payload.replace(close, &format!("{close}{close}"));
            let token = format!("{open}{escaped}{close}");
            check(
                &format!(" /* before */ SELECT  {token} /* after */ ; -- end"),
                &format!("SELECT {token} ;"),
            );
        }
    }
}

#[test]
fn doubled_delimiters_do_not_end_quoted_content() {
    for token in [
        "''",
        "''''",
        "''''''",
        "\"\"",
        "\"\"\"\"",
        "``",
        "````",
        "[]",
        "[]]]",
        "[a]]b]",
        "'a''-- b'",
        "\"a\"\"/*b*/\"",
    ] {
        check(
            &format!("SELECT {token}/*outside*/;"),
            &format!("SELECT {token} ;"),
        );
    }
    check("SELECT 'a'-- removed\n,'b'", "SELECT 'a' ,'b'");
}

#[test]
fn escape_strings_preserve_escaped_quotes_backslashes_and_newlines() {
    for prefix in ["E", "e"] {
        for body in [
            r"a\'  -- b",
            r"a\\",
            r"\n\t\r",
            "\\\n",
            r"\'",
            r"\\\'",
            "é\\'🦀",
            r"''",
            r"\u1234",
        ] {
            let token = format!("{prefix}'{body}'");
            check(
                &format!("SELECT {token}/* outside */;"),
                &format!("SELECT {token} ;"),
            );
        }
        for count in 0..32 {
            let body = "\\".repeat(count);
            let token = if count % 2 == 0 {
                format!("{prefix}'{body}'")
            } else {
                format!("{prefix}'{body}' -- literal\nstill literal'")
            };
            check(&format!("{token}-- outside\n;"), &format!("{token} ;"));
        }
    }
}

#[test]
fn e_prefix_requires_an_identifier_boundary_and_adjacency() {
    for before in ["", " ", "\u{a0}", "\u{2003}", "(", ",", "/**/"] {
        let token = r"E'a\'  -- inside'";
        let expected_prefix = match before {
            "(" | "," => before,
            _ => "",
        };
        check(
            &format!("{before}{token}/* outside */;"),
            &format!("{expected_prefix}{token} ;"),
        );
    }
    for prefix in ["name", "_", "1", "$", "é", "日本語", "🦀"] {
        let token = format!("{prefix}E'C:\\'");
        check(&format!("{token}-- outside\n;"), &format!("{token} ;"));
    }
    for token in [r"'C:\'", r"N'C:\'", r"E 'C:\'", r"e/**/'C:\'"] {
        let expected = token.replace("/**/", " ");
        check(&format!("{token}-- outside\n;"), &format!("{expected} ;"));
    }
}

#[test]
fn dollar_strings_require_exact_case_sensitive_closing_tags() {
    for tag in ["", "tag", "_", "_1", "tag_123", "é", "日本語", "🦀"] {
        let delimiter = format!("${tag}$");
        let payload = "' \" ` [ --\n/* unclosed block  */ \\  $other$ $TAG$ 🦀";
        let token = format!("{delimiter}{payload}{delimiter}");
        check(
            &format!("SELECT  {token}/* outside */;"),
            &format!("SELECT {token} ;"),
        );
    }
    check(
        "SELECT $a$one $ab$ two $A$ three $a$;",
        "SELECT $a$one $ab$ two $A$ three $a$;",
    );
    check("SELECT $tag$$tag$;", "SELECT $tag$$tag$;");
    check("SELECT $$$$;", "SELECT $$$$;");
    check(
        "SELECT ($$-- literal$$),$$/* literal */$$",
        "SELECT ($$-- literal$$),$$/* literal */$$",
    );
}

#[test]
fn dollar_signs_in_identifiers_and_parameters_are_not_quotes() {
    for token in [
        "$1", "$123", "$", "$a", "$1$", "foo$bar", "foo$$", "foo$tag$", "_$tag$", "é$tag$", "🦀$$",
    ] {
        check(
            &format!("SELECT {token}-- outside\n;"),
            &format!("SELECT {token} ;"),
        );
    }
    for white in [' ', '\t', '\u{85}', '\u{a0}', '\u{3000}'] {
        check(
            &format!("SELECT{white}$$  -- inside $${white};"),
            "SELECT $$  -- inside $$ ;",
        );
    }
}

#[test]
fn unicode_whitespace_is_collapsed_only_outside_quotes() {
    let whitespace: Vec<char> = (0..=0x10ffff)
        .filter_map(char::from_u32)
        .filter(|c| c.is_whitespace())
        .collect();
    assert_eq!(whitespace.len(), 25);
    for c in whitespace {
        check(
            &format!("{c}SELECT{c}{c}'{c}{c}'{c};{c}"),
            &format!("SELECT '{c}{c}' ;"),
        );
        check(&format!("SELECT{c}E'a\\' -- b'"), "SELECT E'a\\' -- b'");
        check(
            &format!("SELECT{c}$tag$ -- b $tag$"),
            "SELECT $tag$ -- b $tag$",
        );
    }
    for c in ['\u{200b}', '\u{feff}', '\u{180e}', '\0', 'é', '中', '🦀'] {
        check(&format!("SELECT {c}x{c}"), &format!("SELECT {c}x{c}"));
    }
}

#[test]
fn punctuation_operators_numbers_and_placeholders_are_unchanged() {
    for input in [
        "SELECT 1+2,-3,4.5,6e-7,0xFF;",
        "SELECT * FROM t WHERE a<>b AND c!=d AND e<=f AND g>=h;",
        "SELECT a::text,b->'key',c->>'key',d||e,f&&g;",
        "SELECT ?,:name,@name,$1,$12;",
        "SELECT schema.table,COUNT(*),a/b,a*b,a%b;",
        "SELECT N'-- literal',B'0101',X'CAFE',U&'d\\0061t';",
        "SELECT \"select\",\"two words\",[table name],`column name`;",
        "SELECT '; DROP TABLE users; --' AS value;",
    ] {
        check(input, input);
    }
}

#[test]
fn documented_dialect_limitations_are_explicit() {
    check("SELECT/*+ hint */1/*! executable */;", "SELECT 1 ;");
    check(
        "SELECT # not a supported comment\n1",
        "SELECT # not a supported comment 1",
    );
    check("SELECT 'a'\n'b'", "SELECT 'a' 'b'");
    check(
        "SELECT [1,  /* treated as quoted identifier */ 2]",
        "SELECT [1,  /* treated as quoted identifier */ 2]",
    );
}

#[test]
fn malformed_input_reports_the_construct_instead_of_an_index_panic() {
    let cases = [
        ("/*", "block comment"),
        ("/* /* */", "block comment"),
        ("/* 🦀 *", "block comment"),
        ("'", "quoted string or identifier"),
        ("'''", "quoted string or identifier"),
        ("'é", "quoted string or identifier"),
        ("\"", "quoted string or identifier"),
        ("`", "quoted string or identifier"),
        ("[", "quoted string or identifier"),
        ("[]]", "quoted string or identifier"),
        ("E'\\", "quoted string or identifier"),
        ("E'\\'", "quoted string or identifier"),
        ("$$", "dollar-quoted string"),
        ("$tag$", "dollar-quoted string"),
        ("$a$body$A$", "dollar-quoted string"),
        ("$é$🦀", "dollar-quoted string"),
    ];
    for (input, expected) in cases {
        for emit in [false, true] {
            let panic = std::panic::catch_unwind(|| scan::<256>(input, emit)).expect_err(input);
            let message = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .expect("string panic");
            assert!(message.contains(expected), "{input:?}: {message}");
        }
    }
}

#[test]
fn generated_token_streams_preserve_tokens_and_quote_payloads() {
    // The oracle joins known tokens. It never parses or preprocesses the input.
    let tokens = [
        "SELECT",
        "FROM",
        "table_name",
        "é",
        "🦀",
        "123",
        "$1",
        "?",
        ",",
        ";",
        "(",
        ")",
        "=",
        "::",
        "->>",
        "'a  -- b'",
        "'it''s /*text*/'",
        "\"a\"\"  b\"",
        "`a`` -- b`",
        "[a]] /* b */]",
        "$$a \n -- b$$",
        "$tag$/* a */ $other$  b$tag$",
        r"E'escaped\'  -- b'",
    ];
    let separators = [
        " ",
        "\n\t",
        "/**/",
        "/* -- ' \" */",
        "/* outer /* inner */ */",
        "-- $$ ' /*\r\n",
        "\u{2003}",
    ];
    let mut seed = 0x4829_153a_u64;
    for case in 0..4096 {
        let mut input = String::from("/* leading */");
        let mut expected = String::new();
        for index in 0..(1 + case % 24) {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let token = tokens[(seed >> 32) as usize % tokens.len()];
            if index > 0 {
                expected.push(' ');
            }
            expected.push_str(token);
            input.push_str(token);
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            input.push_str(separators[(seed >> 32) as usize % separators.len()]);
        }
        input.push_str("-- trailing");
        check(&input, &expected);
    }
}

#[test]
fn generated_literal_payloads_are_preserved() {
    let atoms = [
        "a", "é", "🦀", "'", "\"", "`", "]", "\\", "--", "/*", "*/", "\n", "\r", "\t", "\u{3000}",
        "$tag$", "\0",
    ];
    let mut seed = 0x9dea_579c_u64;
    for case in 0..1024 {
        let mut payload = String::new();
        for _ in 0..case % 32 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            payload.push_str(atoms[(seed >> 32) as usize % atoms.len()]);
        }
        for (open, close) in [('\'', '\''), ('"', '"'), ('`', '`'), ('[', ']')] {
            let escaped = payload.replace(close, &format!("{close}{close}"));
            let token = format!("{open}{escaped}{close}");
            check(&format!("/* a */{token}-- b\n;"), &format!("{token} ;"));
        }
        let escaped = payload.replace('\\', "\\\\").replace('\'', "''");
        let token = format!("E'{escaped}'");
        check(&format!("{token}/* outside */;"), &format!("{token} ;"));
        let token = format!("$unique${payload}$unique$");
        check(&format!("{token}/* outside */;"), &format!("{token} ;"));
    }
}

#[test]
fn trimming_copies_exact_bytes_without_normalizing_the_middle() {
    for input in [
        "",
        " ",
        " \t\r\n",
        "  café 🦀  ",
        "\u{3000}a \n b\u{85}",
        "\0 x\0",
        "\u{feff} x \u{200b}",
    ] {
        let expected = input.trim();
        let (start, end) = trim_bounds(input);
        assert_eq!(&input[start..end], expected);
        assert_eq!(trim_len(input), expected.len());
    }
    const INPUT: &str = "\u{3000} café 🦀\n 日本語 \u{85}";
    const LEN: usize = trim_len(INPUT);
    const BYTES: [u8; LEN] = trim::<LEN>(INPUT);
    assert_eq!(as_str(&BYTES), INPUT.trim());
}
