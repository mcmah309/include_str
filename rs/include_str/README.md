# include_str

Include UTF-8 files as static strings, with optional compile-time preprocessing.
No dependencies, no allocation, no runtime processing, and compatible with `no_std`.

```toml
[dependencies]
include_str = "0.0.1"
```

```rust
const RAW: &str = include_str::include_str!("message.txt");
const MESSAGE: &str = include_str::include_str_trim!("message.txt");
const LINES: &str = include_str::include_str_trim_lines!("message.txt");
const QUERY: &str = include_str::include_sql_str!("query.sql");
const CUSTOM: &str = include_str::include_str_replace!("message.txt", "{{name}}", "Rust");
const UNQUOTED: &str = include_str::include_str_strip_prefix!("message.txt", "> ");
const JSON: &str = include_str::include_str_json!("config.json");
const JSON_FROM_JSONC: &str = include_str::include_str_jsonc!("config.jsonc");
```

- `include_str!` is a direct re-export of Rust's built-in macro.
- `include_str_trim!` strips leading and trailing Unicode whitespace, like `str::trim`.
- `include_str_trim_lines!` trims Unicode whitespace from each line, preserving internal whitespace, blank lines, and LF/CRLF line endings (including the final one). A lone carriage return is whitespace, not a line separator.
- `include_sql_str!` removes SQL comments, collapses unquoted Unicode whitespace to single spaces, and trims the result.
- `include_str_replace!` replaces all non-overlapping literal matches, like `str::replace`. An empty search string inserts the replacement at every Unicode character boundary. Arguments must be constant string expressions.
- `include_str_strip_prefix!` removes one matching literal prefix from each line, preserving other content and LF/CRLF endings. It does not trim indentation. An empty prefix has no effect; prefixes containing CR or LF are rejected.
- `include_str_json!` validates and minifies JSON at compile time, preserving strings, escapes, numeric spellings, duplicate keys, and key order. It rejects comments, trailing commas, and BOMs, and supports up to 128 nested arrays/objects. Unicode escapes are validated syntactically, without decoding surrogate pairs.

`include_str_jsonc!` converts JSON with `//` line comments, non-nested `/* ... */`
block comments, and optional trailing commas into minified JSON. Strings and
numeric spellings are preserved. It uses the same validation and 128-container
nesting limit as `include_str_json!`; single quotes, unquoted keys, hexadecimal
numbers, and comments splitting numbers or keywords are rejected. The existing
`include_str_json!` remains strict JSON.

For SQL, this example:

```sql
-- Find active users
SELECT  id, name
FROM /* table */ users
WHERE active = 1;
```

becomes `SELECT id, name FROM users WHERE active = 1;`.

Paths are relative to the invoking Rust source file. Path expressions such as
`concat!(env!("CARGO_MANIFEST_DIR"), "/queries/users.sql")` also work.
Files must exist and contain valid UTF-8. Rust tracks included files so changing
one triggers recompilation.
