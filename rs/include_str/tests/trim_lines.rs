use include_str::include_str_trim_lines;

#[test]
fn includes_and_trims_files_at_compile_time() {
    const TEXT: &str = include_str_trim_lines!("fixtures/unicode.txt",);
    static WINDOWS: &str = include_str_trim_lines!("fixtures/message.txt");
    assert_eq!(TEXT, "café 🦀\n日本語");
    assert_eq!(WINDOWS, "Hello, world!\r\n");
    assert_eq!(include_str_trim_lines!("fixtures/empty.txt"), "");
    assert_eq!(include_str_trim_lines!("fixtures/whitespace.txt"), "\r\n");
    assert_eq!(
        include_str_trim_lines!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/unicode.txt"
        )),
        TEXT
    );
}

macro_rules! check {
    ($input:expr, $expected:expr) => {{
        const LEN: usize = include_str::__private::trim_lines_len($input);
        const BYTES: [u8; LEN] = include_str::__private::trim_lines::<LEN>($input);
        assert_eq!(include_str::__private::as_str(&BYTES), $expected);
    }};
}

#[test]
fn preserves_blank_lines_and_original_line_endings() {
    check!("", "");
    check!(" \t", "");
    check!("\n", "\n");
    check!("\r\n", "\r\n");
    check!(" \n\t\r\n\n ", "\n\r\n\n");
    check!("  a  \n  b  \r\n  c  ", "a\nb\r\nc");
    check!("  a  \n  b  \r\n  c  \n", "a\nb\r\nc\n");
    check!("\r\r\n", "\r\n");
    check!("\ra\rb\r", "a\rb");
    check!(" a\r \n", "a\n");
}

#[test]
fn preserves_internal_whitespace_and_non_whitespace_unicode() {
    check!("  a  \t b  \n  c  d  ", "a  \t b\nc  d");
    check!("\u{3000}café 🦀\u{a0}\n\t日本語\u{85}", "café 🦀\n日本語");
    check!("\u{200b} x \u{feff}", "\u{200b} x \u{feff}");
    check!("\u{2028}a\u{2029}b\u{2028}", "a\u{2029}b");
    check!(" \0 \n \0 ", "\0\n\0");
}
