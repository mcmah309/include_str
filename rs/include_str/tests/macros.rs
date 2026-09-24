use include_str::{include_sql_str, include_str_trim};

#[path = "fixtures/nested/mod.rs"]
mod nested;

const RAW: &str = include_str::include_str!("fixtures/message.txt");
const TRIMMED: &str = include_str_trim!("fixtures/message.txt",);
static SQL: &str = include_sql_str!("fixtures/query.sql",);

#[test]
fn includes_files_at_compile_time() {
    assert_eq!(RAW, " \tHello, world!\r\n");
    assert_eq!(TRIMMED, "Hello, world!");
    assert_eq!(
        SQL,
        "SELECT id, 'not -- a /* comment */' AS label FROM users WHERE active = 1;"
    );
    assert_eq!(nested::VALUE, "nested file");
    assert_eq!(
        include_str_trim!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/message.txt"
        )),
        TRIMMED
    );
    assert_eq!(
        include_sql_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/query.sql"
        )),
        SQL
    );
}

#[test]
fn handles_empty_and_unicode_files() {
    assert_eq!(include_str_trim!("fixtures/empty.txt"), "");
    assert_eq!(include_sql_str!("fixtures/empty.txt"), "");
    assert_eq!(include_str_trim!("fixtures/whitespace.txt"), "");
    assert_eq!(include_sql_str!("fixtures/whitespace.txt"), "");
    assert_eq!(include_sql_str!("fixtures/comments.sql"), "");
    assert_eq!(
        include_str_trim!("fixtures/unicode.txt"),
        "café 🦀\n  日本語"
    );
    assert_eq!(include_sql_str!("fixtures/unicode.txt"), "café 🦀 日本語");
}

// Exercise the same const scanner with small, readable SQL edge cases.
macro_rules! compact {
    ($text:expr) => {{
        const LEN: usize = include_str::__private::sql_len($text);
        const BYTES: [u8; LEN] = include_str::__private::sql::<LEN>($text);
        include_str::__private::as_str(&BYTES)
    }};
}

#[test]
fn preserves_quoted_content() {
    assert_eq!(
        compact!(r#" SELECT 'it''s  -- text', "a""b /* c */", `a``b  c`, [a]]b -- c] "#),
        r#"SELECT 'it''s  -- text', "a""b /* c */", `a``b  c`, [a]]b -- c]"#
    );
    assert_eq!(compact!("SELECT 'a\n \tb';"), "SELECT 'a\n \tb';");
    assert_eq!(
        compact!(r"SELECT E'it\'s  -- text', e'\\';"),
        r"SELECT E'it\'s  -- text', e'\\';"
    );
    assert_eq!(compact!(r"SELECT 'C:\', 1"), r"SELECT 'C:\', 1");
}

#[test]
fn preserves_dollar_quotes_and_parameters() {
    assert_eq!(compact!("SELECT\u{2003}$$a  -- b$$"), "SELECT $$a  -- b$$");
    assert_eq!(
        compact!("SELECT\u{2003}E'a\\'  -- b'"),
        "SELECT E'a\\'  -- b'"
    );
    assert_eq!(
        compact!(" SELECT $$a  -- b\n/* c */$$, $tag_1$x $other$  y$tag_1$ "),
        "SELECT $$a  -- b\n/* c */$$, $tag_1$x $other$  y$tag_1$"
    );
    assert_eq!(
        compact!("SELECT $é$ 🦀  -- x $é$"),
        "SELECT $é$ 🦀  -- x $é$"
    );
    assert_eq!(
        compact!(" SELECT $1, foo$bar, foo$$, $2 "),
        "SELECT $1, foo$bar, foo$$, $2"
    );
}

#[test]
fn separates_tokens_and_handles_line_endings() {
    assert_eq!(
        compact!("/* a */SELECT/**/x/* b */FROM t;-- end"),
        "SELECT x FROM t;"
    );
    assert_eq!(
        compact!("a-- comment\rb-- comment\r\nc-- comment\nd"),
        "a b c d"
    );
    assert_eq!(compact!("a/* outer /* inner */ end */b"), "a b");
    assert_eq!(compact!("a/*comment*/-/*comment*/-b"), "a - -b");
    assert_eq!(compact!("a\u{2003}\u{a0}b"), "a b");
}

#[test]
fn rejects_unterminated_constructs() {
    for input in [
        "/*", "/* /* */", "'x", "\"x", "`x", "[x", "$$x", "$tag$x", "E'x\\",
    ] {
        assert!(
            std::panic::catch_unwind(|| include_str::__private::sql_len(input)).is_err(),
            "{input}"
        );
    }
}

#[test]
fn trimming_matches_standard_library_for_every_unicode_scalar() {
    for scalar in 0..=0x10ffff {
        if let Some(c) = char::from_u32(scalar) {
            let input = format!("{c}x{c}");
            assert_eq!(
                include_str::__private::trim_len(&input),
                input.trim().len(),
                "{scalar:x}"
            );
        }
    }
}

#[test]
fn macros_are_hygienic() {
    const INPUT: &str = "caller";
    const LEN: usize = 123;
    const BYTES: &[u8] = b"caller";
    const TEXT: &str = "caller";
    assert_eq!(include_str_trim!("fixtures/message.txt"), TRIMMED);
    assert_eq!(include_sql_str!("fixtures/query.sql"), SQL);
    assert_eq!(
        (INPUT, LEN, BYTES, TEXT),
        ("caller", 123, b"caller".as_slice(), "caller")
    );
}
