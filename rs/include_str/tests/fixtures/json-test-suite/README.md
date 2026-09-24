# Vendored JSONTestSuite corpus

Source: https://github.com/nst/JSONTestSuite

Pinned revision: `1ef36fa01286573e846ac449e8683f8833c5b26a`

The 318 files in `test_parsing/` are copied byte for byte from that revision.
Their MIT license is included in `LICENSE`. No download is needed to run tests.
Some fixtures intentionally contain invalid UTF-8, NULs, or malformed JSON.
Do not format or normalize them.

Upstream prefixes mean `y_` must accept, `n_` must reject, and `i_` allows
implementation-dependent behavior. This crate's explicit policies for `i_`
cases are tested separately: preserve arbitrary numeric spellings and syntactic
Unicode escapes; reject invalid UTF-8, BOMs, UTF-16, and nesting over 128.
