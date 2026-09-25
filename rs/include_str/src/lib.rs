//! Include UTF-8 files, optionally preprocessing them at compile time.
//!
//! Text macros return a `&'static str`; [`include_lines!`] returns a
//! `&'static [&'static str]`. Paths are relative to the Rust source file
//! containing the invocation, exactly as with Rust's built-in `include_str!`.
//! Files are tracked by the compiler and changes trigger recompilation.
//!
//! ```
//! const RAW: &str = include_str::include_str!("../tests/fixtures/message.txt");
//! const TEXT: &str = include_str::include_str!(
//!     "../tests/fixtures/message.txt" => trim => replace("world", "Rust")
//! );
//! assert_eq!(TEXT, "Hello, Rust!");
//! ```
//!
//! This crate is `no_std`, has no dependencies, and performs no runtime allocation
//! or preprocessing.

#![no_std]

/// Include a UTF-8 file and optionally apply text operations at compile time.
///
/// Operations are separated by `=>` and run left to right. With no operations,
/// this behaves like Rust's built-in `include_str!`.
/// Paths are relative to the invoking source file;
/// `concat!` and `env!` path expressions are supported, as are trailing commas.
/// Every result is a `&'static str`, with no runtime processing or allocation.
///
/// Supported operations:
/// - `trim`: remove leading and trailing Unicode whitespace.
/// - `trim_lines`: trim each line, preserving LF/CRLF endings.
/// - `collapse_whitespace`: collapse runs using newline > tab > space precedence.
/// - `collapse_whitespace(replacement)`: replace each run with a constant string;
///   `collapse_whitespace(" ")` (or `collapse_whitespace(space)`) uses one ASCII space.
/// - `replace(from, to)`: replace literal matches; arguments must be constant strings.
/// - `strip_line_prefix(prefix)`: remove a literal prefix from each matching line.
/// - `strip_line_suffix(suffix)`: remove a literal suffix from each matching line.
/// - `sql`: strip SQL comments and compact unquoted whitespace.
/// - `json`: validate and minify JSON.
/// - `jsonc`: convert JSONC to minified JSON.
///
/// Operations may be reordered or repeated; each processes the previous text
/// and must accept it or compilation fails. They do not convert between formats:
/// `jsonc => sql` applies SQL processing to JSON text. Text operations also affect
/// quoted content. Use `replace(...) => json` to validate after replacement;
/// `json => replace(...)` can invalidate the JSON. See the operation details below.
/// Use [`include_lines!`] separately for a slice of lines instead of text.
/// Whitespace runs include leading/trailing Unicode whitespace. Custom collapse
/// replacements are inserted once per run without being processed again; `""`
/// removes all whitespace.
///
/// ```
/// const TEXT: &str = include_str::include_str!(
///     "../tests/fixtures/message.txt"
///         => collapse_whitespace(" ")
///         => trim
///         => replace("world", "Rust"),
/// );
/// assert_eq!(TEXT, "Hello, Rust!");
/// ```
///
/// ## `trim`
///
/// Remove leading and trailing Unicode whitespace.
///
/// Internal whitespace is preserved. Whitespace has the same definition as
/// [`str::trim`].
///
/// ## `trim_lines`
///
/// Trim Unicode whitespace from each line.
///
/// Preserves whitespace within lines, blank lines, and original LF or CRLF line
/// endings, including a final line ending. A lone carriage return is whitespace,
/// not a line separator. Other Unicode whitespace follows [`str::trim`].
///
/// ## `collapse_whitespace`
///
/// Collapse each whitespace run to one character.
///
/// Newline wins over tab, and tab wins over space, regardless of order. CR and
/// LF (including CRLF) produce `\n`; otherwise a run containing `\t` produces
/// `\t`, and all remaining Unicode whitespace produces an ASCII space.
/// Whitespace has the same definition as [`str::trim`]. Leading and trailing
/// runs are collapsed, not removed; non-whitespace text is preserved.
///
/// ## `collapse_whitespace(replacement)`
///
/// Replace each Unicode whitespace run with a constant string.
///
/// Includes tabs, line endings, and leading/trailing runs. Non-whitespace text
/// is preserved; inserted text is not processed again. `""` removes whitespace,
/// and `" "` (also written `space`) replaces each run with one ASCII space.
///
/// ## `sql`
///
/// Strip SQL comments and collapse unquoted whitespace to one space.
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
/// Malformed SQL fails during constant evaluation.
///
/// ```compile_fail,E0080
/// const SQL: &str = include_str::include_str!("../tests/fixtures/broken.sql" => sql);
/// ```
///
/// ## `replace`
///
/// Replace every non-overlapping literal occurrence.
///
/// Matches are processed left to right, like `str::replace`. Replacement text
/// is not searched again within the same pair. An empty search string inserts the replacement at
/// every Unicode character boundary, including the start and end.
/// Chain `replace(from, to)` operations to apply multiple pairs in order; later
/// operations also match text inserted by earlier ones.
/// Search and replacement arguments must be constant string expressions.
///
/// ## `strip_line_prefix`
///
/// Remove one literal prefix from each matching line.
///
/// Lines without the prefix are unchanged. Matching starts at the first byte of
/// each line without trimming indentation. LF and CRLF endings are preserved;
/// a lone CR does not start a new line. An empty prefix has no effect.
/// The prefix must be a constant string expression and cannot contain CR or LF.
///
/// ## `strip_line_suffix`
///
/// Remove one literal suffix from each matching line.
///
/// Matches immediately before the LF/CRLF ending or the end of the file, without
/// trimming whitespace. Line endings and nonmatching lines are preserved; a lone
/// CR is content, not a line separator. An empty suffix has no effect.
/// The suffix must be a constant string expression and cannot contain CR or LF.
///
/// ## `json`
///
/// Validate JSON syntax and remove formatting whitespace.
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
/// ## `jsonc`
///
/// Convert JSONC to minified, valid JSON.
///
/// Accepts `//` line comments, non-nested `/* ... */` block comments, and a
/// single trailing comma in nonempty arrays or objects. Comments are allowed
/// wherever JSON whitespace is allowed, but cannot split a number or keyword.
/// Line comments end at CR, LF, or the end of the file.
///
/// Strings (including comment markers inside them), escapes, numbers, key order,
/// and duplicate keys are preserved. All other validation rules and the nesting
/// limit of 128 containers match `json`. This is not JSON5:
/// single quotes, unquoted keys, and hexadecimal numbers are rejected.
///
#[macro_export]
macro_rules! include_str {
    ($path:expr $(,)?) => {
        $crate::__private::raw_include_str!($path)
    };
    ($path:expr $(=> $op:ident $(($($args:tt)*))?)+ $(,)?) => {
        $crate::__include_str_pipeline!(
            $crate::__private::raw_include_str!($path);
            $($op $(($($args)*))?),+
        )
    };
    ($($invalid:tt)*) => {
        ::core::compile_error!("include_str!: expected a path followed by operations separated by =>")
    };
}

mod pipeline;

/// Include a UTF-8 file as a static slice of lines for lookup tables or word lists.
///
/// Splits processed text like [`str::lines`]: LF and CRLF delimiters are removed,
/// blank lines are preserved, and a final line ending adds no extra entry. An
/// empty file produces an empty slice. Lone CRs, other Unicode whitespace,
/// indentation, duplicates, and line order are preserved unless changed by an
/// operation. Both the slice and its strings have static lifetimes.
///
/// Accepts the same paths and `=>` operations as [`include_str!`]. Operations run
/// on the whole text before splitting, not independently on each line. Use
/// `trim_lines` to trim each line. Processing and splitting happen at compile
/// time; entries borrow from the resulting static text without runtime allocation.
///
/// ```
/// const WORDS: &[&str] = include_str::include_lines!("../tests/fixtures/words.txt");
/// assert_eq!(WORDS, &["apple", "banana", "cherry"]);
/// assert!(WORDS.contains(&"banana"));
/// // This fixture is already sorted, so binary search is also available.
/// assert_eq!(WORDS.binary_search(&"cherry"), Ok(2));
///
/// const LINES: &[&str] = include_str::include_lines!(
///     "../tests/fixtures/message.txt" => trim_lines => replace("world", "Rust")
/// );
/// assert_eq!(LINES, &["Hello, Rust!"]);
/// ```
#[macro_export]
macro_rules! include_lines {
    ($($input:tt)*) => {{
        const INPUT: &str = $crate::include_str!($($input)*);
        const LEN: usize = $crate::__private::lines_len(INPUT);
        const LINES: [&str; LEN] = $crate::__private::lines::<LEN>(INPUT);
        const RESULT: &[&str] = &LINES;
        RESULT
    }};
}

// Public only so exported macros can use these functions in downstream crates.
#[doc(hidden)]
pub mod __private;
