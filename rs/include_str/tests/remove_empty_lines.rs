use include_str::{__private, include_lines, include_str};

macro_rules! check {
    ($input:expr, $expected:expr) => {{
        const LEN: usize = __private::remove_empty_lines_len($input);
        const BYTES: [u8; LEN] = __private::remove_empty_lines::<LEN>($input);
        assert_eq!(__private::as_str(&BYTES), $expected);
    }};
}

#[test]
fn preserves_content_and_line_endings() {
    check!("", "");
    check!("\n\r\n\n", "");
    check!("\na\r\n\r\nb\n\nc", "a\r\nb\nc");
    check!("a\n\n", "a\n");
    check!(" \n\t\r\n\u{3000}\n", " \n\t\r\n\u{3000}\n");
    check!("\r", "\r");
    check!("\r\r\n\n\r", "\r\r\n\r");
    check!("\né🦀\r\n\n\0", "é🦀\r\n\0");
}

#[test]
fn both_macros_support_ordered_pipelines() {
    const SOURCE: &str = "\n apple \r\n \n\r\nbanana\n\n";
    const TEXT: &str =
        include_str!("fixtures/empty.txt" => replace("", SOURCE) => remove_empty_lines);
    assert_eq!(TEXT, " apple \r\n \nbanana\n");
    static WORDS: &[&str] = include_lines!(
        "fixtures/empty.txt" => replace("", SOURCE) => trim_lines => remove_empty_lines,
    );
    assert_eq!(WORDS, &["apple", "banana"]);
    assert_eq!(
        include_lines!("fixtures/empty.txt" => replace("", SOURCE) => remove_empty_lines => trim_lines),
        &["apple", "", "banana"]
    );
    assert_eq!(
        include_lines!("fixtures/empty.txt" => remove_empty_lines),
        &[] as &[&str]
    );
}
