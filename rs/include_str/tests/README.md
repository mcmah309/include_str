# Test coverage

Run the complete suite offline with:

```sh
cargo test --workspace
cargo +1.85.0 test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The compiler integration tests require `rustc` on `PATH` (or `RUSTC` set).
Temporary consumers are isolated and removed after each test. No database,
network connection, test dependencies, or external JSON interpreter is required.

## JSON and JSONC

- All 318 vendored [JSONTestSuite](fixtures/json-test-suite/README.md) fixtures,
  including explicit expectations for implementation-dependent cases.
- All upstream `y_` files compiled through both public macros; invalid UTF-8
  corpus files rejected through both public macros.
- All 65,536 four-digit Unicode escapes; each ASCII character after a backslash;
  each ASCII character in every Unicode-escape digit position.
- Raw controls, permitted whitespace, malformed numbers, escapes, surrogate
  spellings, duplicate keys, and exact preservation of string/number bytes.
- Truncation at character boundaries, incomplete comments, token splitting,
  root concatenation, comma placement, and comments at grammar boundaries.
- Pure-object, array, and mixed nesting at/beyond 128; wide sibling collections.
- Thousands of generated JSON/JSONC documents with independently constructed
  expected output. Successful JSONC output must also pass strict validation.

## Text and SQL

- Pipeline operations checked in all 81 ordered pairs against independently
  constructed results. Longer chains exercise repeated replacements, ordering,
  constant arguments, empty results, and JSONC-to-JSON validation.
- `=>` pipeline syntax checked with literal and macro-generated paths; old comma
  separators, mixed separators, and incomplete pipelines are rejected.
- Compiler diagnostics checked with each operation before and after failing
  SQL/JSON/JSONC validation and invalid prefixes, plus unknown operations at
  the beginning, middle, and end. JSON and JSONC errors identify their own mode.
- Line tables compared with `str::lines()` over every Unicode scalar and all
  short mixed-text inputs, including LF/CRLF boundaries, blank lines, lone CRs,
  borrowed entries, lookup operations, and thousands of entries.

- All strings up to five characters over a seven-character alphabet containing
  ASCII, multibyte Unicode, spaces, tabs, CR, and LF. Replacement, trimming, and
  prefix stripping are compared with standard-library operations.
- Every Unicode scalar tested for trimming and UTF-8-safe replacement/prefix
  removal; all Unicode whitespace characters checked in SQL quoting contexts.
- SQL quote escapes, dollar-tag boundaries, truncated quotes, nested comments,
  token separation, generated quoted payloads, and full SQL file fixtures.
- Every public macro tested with a renamed dependency, `no_std`, constants,
  statics, expression contexts, Unicode paths containing spaces, caller-name
  collisions, missing files, invalid UTF-8, and malformed argument lists.

These tests check the documented behavior. They do not prove correctness for
every possible input or extend support to undocumented SQL dialects/JSON5.
