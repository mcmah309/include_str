//! Include UTF-8 files, optionally preprocessing them at compile time.
//!
//! All macros return a `&'static str`. Paths are relative to the Rust source file
//! containing the invocation, exactly as with Rust's built-in `include_str!`.
//! Files are tracked by the compiler and changes trigger recompilation.
//!
//! ```
//! const RAW: &str = include_str::include_str!("../tests/fixtures/message.txt");
//! const TRIMMED: &str = include_str::include_str_trim!("../tests/fixtures/message.txt");
//! assert_eq!(TRIMMED, "Hello, world!");
//! ```
//!
//! This crate is `no_std`, has no dependencies, and performs no runtime allocation
//! or preprocessing.

#![no_std]

/// Rust's built-in macro, re-exported without changing its behavior.
pub use core::include_str;

/// Include a UTF-8 file and remove leading and trailing Unicode whitespace.
///
/// Internal whitespace is preserved. Whitespace has the same definition as
/// [`str::trim`]. Accepts the same path expressions as [`include_str!`], including
/// `concat!` and `env!`.
#[macro_export]
macro_rules! include_str_trim {
    ($path:expr $(,)?) => {{
        const INPUT: &str = $crate::include_str!($path);
        const LEN: usize = $crate::__private::trim_len(INPUT);
        const BYTES: [u8; LEN] = $crate::__private::trim::<LEN>(INPUT);
        const TEXT: &str = $crate::__private::as_str(&BYTES);
        TEXT
    }};
}

/// Include a UTF-8 file and trim Unicode whitespace from each line.
///
/// Preserves whitespace within lines, blank lines, and original LF or CRLF line
/// endings, including a final line ending. A lone carriage return is whitespace,
/// not a line separator. Other Unicode whitespace follows [`str::trim`].
/// Accepts the same path expressions as [`include_str!`].
///
/// ```
/// const TEXT: &str = include_str::include_str_trim_lines!("../tests/fixtures/message.txt");
/// assert_eq!(TEXT, "Hello, world!\r\n");
/// ```
#[macro_export]
macro_rules! include_str_trim_lines {
    ($path:expr $(,)?) => {{
        const INPUT: &str = $crate::include_str!($path);
        const LEN: usize = $crate::__private::trim_lines_len(INPUT);
        const BYTES: [u8; LEN] = $crate::__private::trim_lines::<LEN>(INPUT);
        const TEXT: &str = $crate::__private::as_str(&BYTES);
        TEXT
    }};
}

/// Include SQL, strip comments, and collapse unquoted whitespace to one space.
///
/// Removes `--` line comments and `/* ... */` block comments (including nested
/// blocks). Comments separate tokens, just like whitespace; leading and trailing
/// whitespace is removed. Unicode whitespace is recognized.
///
/// Single-quoted strings, double-quoted identifiers, backtick identifiers,
/// bracket identifiers, and PostgreSQL `$$...$$` / `$tag$...$tag$` strings are
/// preserved byte for byte. Doubled quote delimiters are supported, as are
/// backslash escapes in PostgreSQL `E'...'` strings. Unterminated quotes or block
/// comments produce a compile-time error.
///
/// This is a lexical compactor, not a SQL parser or a universal dialect adapter.
/// It does not support MySQL `#` comments or implicit backslash escapes in plain
/// strings. All block comments are removed, including optimizer hints and MySQL
/// executable comments. SQL that depends on comments or significant unquoted
/// newlines (such as PostgreSQL newline-separated adjacent literals) should use
/// [`include_str!`] instead. Brackets are always treated as quoted identifiers,
/// so use the raw macro for dialects that use brackets for array expressions.
///
/// ```
/// const SQL: &str = include_str::include_sql_str!("../tests/fixtures/query.sql");
/// assert_eq!(SQL, "SELECT id, 'not -- a /* comment */' AS label FROM users WHERE active = 1;");
/// ```
///
/// Malformed SQL fails during constant evaluation:
///
/// ```compile_fail,E0080
/// const SQL: &str = include_str::include_sql_str!("../tests/fixtures/broken.sql");
/// ```
#[macro_export]
macro_rules! include_sql_str {
    ($path:expr $(,)?) => {{
        const INPUT: &str = $crate::include_str!($path);
        const LEN: usize = $crate::__private::sql_len(INPUT);
        const BYTES: [u8; LEN] = $crate::__private::sql::<LEN>(INPUT);
        const TEXT: &str = $crate::__private::as_str(&BYTES);
        TEXT
    }};
}

/// Include a UTF-8 file and replace every non-overlapping literal occurrence.
///
/// Matches are processed left to right, like `str::replace`. Replacement text
/// is not searched again. An empty search string inserts the replacement at
/// every Unicode character boundary, including the start and end.
/// Search and replacement arguments must be constant string expressions.
///
/// ```
/// const TEXT: &str = include_str::include_str_replace!(
///     "../tests/fixtures/message.txt", "world", "Rust",
/// );
/// assert_eq!(TEXT, " \tHello, Rust!\r\n");
/// ```
#[macro_export]
macro_rules! include_str_replace {
    ($path:expr, $from:expr, $to:expr $(,)?) => {
        const {
            $crate::__private::as_str(
                &const {
                    $crate::__private::replace::<
                        { $crate::__private::replace_len($crate::include_str!($path), $from, $to) },
                    >($crate::include_str!($path), $from, $to)
                },
            )
        }
    };
}

/// Include a UTF-8 file and remove one literal prefix from each matching line.
///
/// Lines without the prefix are unchanged. Matching starts at the first byte of
/// each line without trimming indentation. LF and CRLF endings are preserved;
/// a lone CR does not start a new line. An empty prefix has no effect.
/// The prefix must be a constant string expression and cannot contain CR or LF.
///
/// ```
/// const TEXT: &str = include_str::include_str_strip_prefix!(
///     "../tests/fixtures/message.txt", " \t",
/// );
/// assert_eq!(TEXT, "Hello, world!\r\n");
/// ```
#[macro_export]
macro_rules! include_str_strip_prefix {
    ($path:expr, $prefix:expr $(,)?) => {
        const {
            $crate::__private::as_str(
                &const {
                    $crate::__private::strip_prefix::<
                        {
                            $crate::__private::strip_prefix_len(
                                $crate::include_str!($path),
                                $prefix,
                            )
                        },
                    >($crate::include_str!($path), $prefix)
                },
            )
        }
    };
}

/// Include a UTF-8 JSON file, validate its syntax, and remove formatting whitespace.
///
/// Preserves strings, escape spellings, numbers, key order, and duplicate keys
/// byte for byte. Accepts any JSON root value. Only space, tab, CR, and LF outside
/// strings are removed. Comments, trailing commas, BOMs, and malformed input
/// produce compile-time errors. Nesting is limited to 128 arrays/objects.
///
/// Follows the [RFC 8259 grammar](https://www.rfc-editor.org/rfc/rfc8259):
/// Unicode escapes are checked syntactically but not decoded, so unpaired
/// surrogate escapes are preserved. Numbers are not restricted to float ranges.
///
/// ```
/// const JSON: &str = include_str::include_str_json!("../tests/fixtures/config.json");
/// assert_eq!(JSON, r#"{"message":"two  spaces","enabled":true,"values":[1,null]}"#);
/// ```
#[macro_export]
macro_rules! include_str_json {
    ($path:expr $(,)?) => {{
        const INPUT: &str = $crate::include_str!($path);
        const LEN: usize = $crate::__private::json_len(INPUT);
        const BYTES: [u8; LEN] = $crate::__private::json::<LEN>(INPUT);
        const TEXT: &str = $crate::__private::as_str(&BYTES);
        TEXT
    }};
}

// Public only so exported macros can use these functions in downstream crates.
#[doc(hidden)]
pub mod __private;
