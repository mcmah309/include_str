use include_str::{__private, include_lines};

#[test]
fn lookup_table_is_available_in_constants_statics_and_expressions() {
    const WORDS: &[&str] = include_lines!("fixtures/words.txt",);
    static STATIC_WORDS: &[&str] = include_lines!("fixtures/words.txt");
    fn words() -> &'static [&'static str] {
        include_lines!("fixtures/words.txt")
    }
    assert_eq!(WORDS, &["apple", "banana", "cherry"]);
    assert_eq!(STATIC_WORDS, WORDS);
    assert_eq!(words(), WORDS);
    assert!(WORDS.contains(&"banana"));
    assert!(!WORDS.contains(&"pear"));
    assert_eq!(WORDS.binary_search(&"cherry"), Ok(2));
    assert_eq!(
        include_lines!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/words.txt"
        )),
        WORDS
    );
    assert_eq!(include_lines!("fixtures/empty.txt"), &[] as &[&str]);
}

macro_rules! check {
    ($input:expr, $expected:expr) => {{
        const INPUT: &str = $input;
        const LEN: usize = __private::lines_len(INPUT);
        const LINES: [&str; LEN] = __private::lines::<LEN>(INPUT);
        assert_eq!(LINES.as_slice(), $expected, "{INPUT:?}");
        assert_eq!(LINES.as_slice(), INPUT.lines().collect::<Vec<_>>());
    }};
}

#[test]
fn empty_lines_and_all_line_ending_boundaries_match_str_lines() {
    check!("", &[] as &[&str]);
    check!("\n", &[""]);
    check!("\n\n", &["", ""]);
    check!("\r\n", &[""]);
    check!("\r\n\r\n", &["", ""]);
    check!("a", &["a"]);
    check!("a\n", &["a"]);
    check!("a\n\n", &["a", ""]);
    check!("\na\n", &["", "a"]);
    check!("a\r\nb\nc", &["a", "b", "c"]);
    check!("\r", &["\r"]);
    check!("a\rb\r", &["a\rb\r"]);
    check!("\r\r\n", &["\r"]);
    check!("\n\r", &["", "\r"]);
    check!("a\n \n", &["a", " "]);
}

#[test]
fn whitespace_duplicates_order_and_non_ascii_text_are_preserved() {
    assert_eq!(
        include_lines!("fixtures/lookup_mixed.txt"),
        &["apple", "", " banana ", "🦀\rapple", "apple"]
    );
    check!(
        " \t é \u{3000}\n\0🦀\n\u{feff}\u{200b}",
        &[" \t é \u{3000}", "\0🦀", "\u{feff}\u{200b}"]
    );
    check!("a\u{85}b\u{2028}c\u{2029}", &["a\u{85}b\u{2028}c\u{2029}"]);
    check!("z\na\nz", &["z", "a", "z"]);
    // No parsing, comment removal or trimming is implied by line inclusion.
    check!(
        "  //comment\n#comment\n/*text*/",
        &["  //comment", "#comment", "/*text*/"]
    );
}

fn compare_with_std(input: &str) {
    let expected: Vec<_> = input.lines().collect();
    assert_eq!(__private::lines_len(input), expected.len(), "{input:?}");
    macro_rules! compare {
        ($n:literal) => {{
            let actual = __private::lines::<$n>(input);
            assert_eq!(actual.as_slice(), expected, "{input:?}");
            // Nonempty entries are borrowed substrings, not copied allocations.
            for line in actual {
                if !line.is_empty() {
                    let offset = line.as_ptr() as usize - input.as_ptr() as usize;
                    assert!(offset + line.len() <= input.len());
                }
            }
        }};
    }
    match expected.len() {
        0 => compare!(0),
        1 => compare!(1),
        2 => compare!(2),
        3 => compare!(3),
        4 => compare!(4),
        5 => compare!(5),
        _ => panic!("test input exceeded five lines"),
    }
}

#[test]
fn exhaustive_short_inputs_match_standard_library() {
    let atoms = ["a", "é", "🦀", " ", "\t", "\r", "\n"];
    for size in 0..=5 {
        for mut combination in 0..atoms.len().pow(size) {
            let mut input = String::new();
            for _ in 0..size {
                input.push_str(atoms[combination % atoms.len()]);
                combination /= atoms.len();
            }
            compare_with_std(&input);
        }
    }
}

#[test]
fn every_unicode_scalar_is_preserved_and_only_lf_splits_lines() {
    for scalar in 0..=0x10ffff {
        if let Some(c) = char::from_u32(scalar) {
            compare_with_std(&format!("{c}x{c}"));
        }
    }
}

#[test]
fn many_entries_have_no_artificial_line_limit() {
    let input = "word\r\n".repeat(4096);
    let lines = __private::lines::<4096>(&input);
    assert_eq!(__private::lines_len(&input), 4096);
    assert!(lines.iter().all(|&line| line == "word"));
}
