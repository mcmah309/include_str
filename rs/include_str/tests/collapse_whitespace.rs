use include_str::include_str;

#[test]
fn includes_files_at_compile_time() {
    const TEXT: &str = include_str!("fixtures/message.txt" => collapse_whitespace);
    static ABSOLUTE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/message.txt"
    ) => collapse_whitespace);
    assert_eq!(TEXT, "\tHello, world!\n");
    assert_eq!(ABSOLUTE, TEXT);
    assert_eq!(
        include_str!("fixtures/empty.txt" => collapse_whitespace),
        ""
    );
}

macro_rules! check {
    ($input:expr, $expected:expr) => {{
        const LEN: usize = include_str::__private::collapse_whitespace_len($input);
        const BYTES: [u8; LEN] = include_str::__private::collapse_whitespace::<LEN>($input);
        assert_eq!(include_str::__private::as_str(&BYTES), $expected);
    }};
}

#[test]
fn strongest_whitespace_wins_in_either_order() {
    check!("\n \t", "\n");
    check!("     \t", "\t");
    check!("\t \n", "\n");
    check!("\t     ", "\t");
    check!("a   b\t \tc\n\n \td", "a b\tc\nd");
    check!("\r\n \t\r\n", "\n");
    check!("a\rb", "a\nb");
}

#[test]
fn handles_boundaries_and_unicode() {
    check!("", "");
    check!("   ", " ");
    check!(" \t café 🦀  日本語 \n ", "\tcafé 🦀 日本語\n");
    check!("\u{a0}\u{1680}\u{2000}\u{3000}", " ");
    check!("\u{a0}\t\u{3000}", "\t");
    check!("\u{3000}\n\u{a0}", "\n");
    check!("\u{b}\u{c}\u{85}\u{2028}\u{2029}", " ");
    check!("\u{200b}\u{feff}\0café🦀", "\u{200b}\u{feff}\0café🦀");
}
