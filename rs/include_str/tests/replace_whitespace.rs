use include_str::include_str;

#[test]
fn includes_files_at_compile_time() {
    const TEXT: &str = include_str!("fixtures/message.txt" => replace_whitespace(" "));
    static ABSOLUTE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/message.txt"
    ) => replace_whitespace(" "));
    assert_eq!(TEXT, " Hello, world! ");
    assert_eq!(ABSOLUTE, TEXT);
    assert_eq!(
        include_str!("fixtures/empty.txt" => replace_whitespace(" ")),
        ""
    );
    assert_eq!(
        include_str!("fixtures/whitespace.txt" => replace_whitespace(" ")),
        " "
    );
}

macro_rules! check {
    ($input:expr, $expected:expr) => {{
        const LEN: usize = include_str::__private::replace_whitespace_len($input, " ");
        const BYTES: [u8; LEN] = include_str::__private::replace_whitespace::<LEN>($input, " ");
        assert_eq!(include_str::__private::as_str(&BYTES), $expected);
    }};
}

#[test]
fn collapses_tabs_newlines_and_boundary_runs_to_spaces() {
    check!("", "");
    check!("     ", " ");
    check!("\n \t", " ");
    check!("     \t", " ");
    check!("\t \r\n\n", " ");
    check!(" \t a   b\t\tc\r\nd\re\n ", " a b c d e ");
    check!("a b", "a b");
}

#[test]
fn recognizes_unicode_whitespace_and_preserves_other_characters() {
    check!("\u{a0}\u{1680}\u{2000}\u{3000}", " ");
    check!("\u{b}\u{c}\u{85}\u{2028}\u{2029}", " ");
    check!("\u{3000}café\t🦀\n日本語\u{a0}", " café 🦀 日本語 ");
    check!("\u{200b}\u{feff}\0café🦀", "\u{200b}\u{feff}\0café🦀");
}
