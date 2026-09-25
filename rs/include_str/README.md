# include_str

Include UTF-8 files as static strings, with optional compile-time preprocessing.
No dependencies, no allocation, no runtime processing, and compatible with `no_std`.

```toml
[dependencies]
include_str = "0.0.1"
```

```rust
const RAW: &str = include_str::include_str!("message.txt");
const CONFIG: &str = include_str::include_str!(
    "config.jsonc" => replace("{{name}}", "Rust") => jsonc
);
const WORDS: &[&str] = include_str::include_lines!("words.txt" => trim_lines);
```

Separate operations with `=>`. They run **left to right**, each receiving the
previous step's text. Reorder or repeat operations as needed; every step must
accept its input or compilation fails. The result is always a `&'static str`.
With no operations, the file is included unchanged.

Order matters: `replace(...) => jsonc` validates and minifies the replaced text;
`jsonc => replace(...)` replaces text after validation, so the final result may
no longer be valid JSON. Text operations also affect quoted content. Operations
process text, not typed values: `jsonc => sql` applies SQL processing to the JSON
text; it does not convert JSON into SQL.

| Operation | Effect |
| --- | --- |
| `trim` | Remove leading and trailing Unicode whitespace. |
| `trim_lines` | Trim each line, preserving LF/CRLF endings. |
| `collapse_whitespace` | Collapse each whitespace run using newline > tab > space precedence. |
| `collapse_whitespace(replacement)` | Replace each whitespace run with a string, e.g. `" "`, `" / "`, or `""`. |
| `replace(from, to)` | Replace all non-overlapping literal matches. |
| `strip_line_prefix(prefix)` | Remove one matching prefix from each line, preserving line endings. |
| `strip_line_suffix(suffix)` | Remove one matching suffix from each line, preserving line endings. |
| `sql` | Strip SQL comments, compact unquoted whitespace, and trim. |
| `json` | Validate strict JSON and remove whitespace outside strings. |
| `jsonc` | Accept comments and trailing commas, producing minified JSON. |

Replacement, prefix, and suffix arguments must be constant string expressions.
`collapse_whitespace(" ")` also accepts the shorthand `collapse_whitespace(space)`.
Collapse replaces leading/trailing Unicode whitespace runs too; inserted text
is not processed again, and `""` removes all whitespace. Replacement
text is not searched again within the same step; an empty search string inserts
text at every Unicode character boundary. Prefixes and suffixes cannot contain
CR or LF. Suffixes match just before LF/CRLF or the end of the file, without trimming.
The pipeline and operation argument lists accept trailing commas.

JSON operations preserve strings, escapes, numeric spellings, duplicate keys,
and key order. Nesting is limited to 128 arrays/objects; Unicode escapes are
checked without decoding surrogate pairs. `jsonc` accepts `//` and non-nested
`/* ... */` comments and a trailing comma in nonempty arrays/objects, but not
JSON5 features such as single quotes or unquoted keys.

SQL processing preserves quoted content and supports nested block comments.
It is a lexical compactor, not a full SQL parser or dialect converter.

`include_str!` and `include_lines!` are the two entry points. The `include_str!`
API docs describe each operation in detail. `include_lines!` accepts the same
pipeline, processing the whole text **before splitting** into a
`&'static [&'static str]`. Operations
do not run independently on each line; use `trim_lines` to trim each line.
Splitting follows `str::lines()`: LF/CRLF endings are removed, blank lines are
kept, and a final line ending adds no extra entry.

Paths are relative to the invoking Rust source file. Path expressions such as
`concat!(env!("CARGO_MANIFEST_DIR"), "/queries/users.sql")` also work.
Files must exist and contain valid UTF-8. Rust tracks included files so changing
one triggers recompilation.
