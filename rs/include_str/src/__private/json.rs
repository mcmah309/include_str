//! JSON grammar validation and minification. Strings and numbers are copied
//! verbatim; no decoding, floating-point conversion, or key reordering occurs.

// Keep the operation name accurate even for grammar errors shared by JSON/JSONC.
macro_rules! json_error {
    ($jsonc:expr, $message:literal) => {
        if $jsonc {
            panic!(concat!("include_str!: jsonc: ", $message))
        } else {
            panic!(concat!("include_str!: json: ", $message))
        }
    };
}

macro_rules! json_assert {
    ($jsonc:expr, $condition:expr, $message:literal) => {
        if !$condition {
            json_error!($jsonc, $message);
        }
    };
}

struct Parser<'a, const N: usize> {
    input: &'a [u8],
    at: usize,
    output: [u8; N],
    written: usize,
    emit: bool,
    jsonc: bool,
}

impl<const N: usize> Parser<'_, N> {
    const fn whitespace(&mut self) {
        loop {
            while self.at < self.input.len()
                && matches!(self.input[self.at], b' ' | b'\t' | b'\r' | b'\n')
            {
                self.at += 1;
            }
            if !self.jsonc || self.at + 1 >= self.input.len() || self.input[self.at] != b'/' {
                return;
            }
            match self.input[self.at + 1] {
                b'/' => {
                    self.at += 2;
                    while self.at < self.input.len()
                        && !matches!(self.input[self.at], b'\r' | b'\n')
                    {
                        self.at += 1;
                    }
                }
                b'*' => {
                    self.at += 2;
                    while self.at + 1 < self.input.len()
                        && !(self.input[self.at] == b'*' && self.input[self.at + 1] == b'/')
                    {
                        self.at += 1;
                    }
                    json_assert!(
                        self.jsonc,
                        self.at + 1 < self.input.len(),
                        "unterminated block comment"
                    );
                    self.at += 2;
                }
                _ => return,
            }
        }
    }

    // Delay emitting the comma until we know it is not a JSONC trailing comma.
    const fn comma(&mut self, close: u8) -> bool {
        json_assert!(
            self.jsonc,
            self.peek() == b',',
            "expected comma or closing delimiter"
        );
        self.at += 1;
        if self.jsonc {
            self.whitespace();
            if self.peek() == close {
                return true;
            }
        }
        if self.emit {
            self.output[self.written] = b',';
        }
        self.written += 1;
        false
    }

    const fn peek(&self) -> u8 {
        json_assert!(
            self.jsonc,
            self.at < self.input.len(),
            "unexpected end of JSON"
        );
        self.input[self.at]
    }

    const fn copy_to(&mut self, end: usize) {
        while self.at < end {
            if self.emit {
                self.output[self.written] = self.input[self.at];
            }
            self.written += 1;
            self.at += 1;
        }
    }

    const fn punctuation(&mut self, expected: u8) {
        json_assert!(
            self.jsonc,
            self.peek() == expected,
            "unexpected JSON punctuation"
        );
        self.copy_to(self.at + 1);
    }

    const fn string(&mut self) {
        json_assert!(
            self.jsonc,
            self.peek() == b'"',
            "object keys must be strings"
        );
        let mut i = self.at + 1;
        while i < self.input.len() {
            let byte = self.input[i];
            if byte == b'"' {
                self.copy_to(i + 1);
                return;
            }
            json_assert!(
                self.jsonc,
                byte >= 0x20,
                "unescaped control character in string"
            );
            if byte == b'\\' {
                i += 1;
                json_assert!(self.jsonc, i < self.input.len(), "incomplete string escape");
                match self.input[i] {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                    b'u' => {
                        let mut digit = 0;
                        while digit < 4 {
                            i += 1;
                            json_assert!(
                                self.jsonc,
                                i < self.input.len() && self.input[i].is_ascii_hexdigit(),
                                "invalid Unicode escape"
                            );
                            digit += 1;
                        }
                    }
                    _ => json_error!(self.jsonc, "invalid string escape"),
                }
            }
            i += 1;
        }
        json_error!(self.jsonc, "unterminated JSON string");
    }

    const fn literal(&mut self, literal: &[u8]) {
        let mut i = 0;
        while i < literal.len() {
            json_assert!(
                self.jsonc,
                self.at + i < self.input.len() && self.input[self.at + i] == literal[i],
                "invalid JSON literal"
            );
            i += 1;
        }
        self.copy_to(self.at + literal.len());
    }

    const fn number(&mut self) {
        let mut i = self.at;
        if self.input[i] == b'-' {
            i += 1;
        }
        json_assert!(
            self.jsonc,
            i < self.input.len() && self.input[i].is_ascii_digit(),
            "invalid JSON number"
        );
        if self.input[i] == b'0' {
            i += 1;
        } else {
            while i < self.input.len() && self.input[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i < self.input.len() && self.input[i] == b'.' {
            i += 1;
            let start = i;
            while i < self.input.len() && self.input[i].is_ascii_digit() {
                i += 1;
            }
            json_assert!(self.jsonc, i > start, "missing fractional digits");
        }
        if i < self.input.len() && matches!(self.input[i], b'e' | b'E') {
            i += 1;
            if i < self.input.len() && matches!(self.input[i], b'+' | b'-') {
                i += 1;
            }
            let start = i;
            while i < self.input.len() && self.input[i].is_ascii_digit() {
                i += 1;
            }
            json_assert!(self.jsonc, i > start, "missing exponent digits");
        }
        self.copy_to(i);
    }
}

// Explicit grammar stack avoids recursion in both const evaluation and tests.
// States: root value/end, array first/value/end, object first/key/colon/value/end.
const fn scan<const N: usize>(input: &str, emit: bool, jsonc: bool) -> ([u8; N], usize) {
    let mut parser = Parser {
        input: input.as_bytes(),
        at: 0,
        output: [0; N],
        written: 0,
        emit,
        jsonc,
    };
    let mut states = [0u8; 129];
    let mut depth = 0;
    loop {
        parser.whitespace();
        match states[depth] {
            1 => {
                json_assert!(
                    parser.jsonc,
                    parser.at == parser.input.len(),
                    "trailing content after JSON value"
                );
                return (parser.output, parser.written);
            }
            2 if parser.peek() == b']' => {
                parser.punctuation(b']');
                depth -= 1;
                continue;
            }
            4 => {
                if parser.peek() == b']' {
                    parser.punctuation(b']');
                    depth -= 1;
                } else {
                    if parser.comma(b']') {
                        parser.punctuation(b']');
                        depth -= 1;
                    } else {
                        states[depth] = 3;
                    }
                }
                continue;
            }
            5 if parser.peek() == b'}' => {
                parser.punctuation(b'}');
                depth -= 1;
                continue;
            }
            5 | 6 => {
                parser.string();
                states[depth] = 7;
                continue;
            }
            7 => {
                parser.punctuation(b':');
                states[depth] = 8;
                continue;
            }
            9 => {
                if parser.peek() == b'}' {
                    parser.punctuation(b'}');
                    depth -= 1;
                } else {
                    if parser.comma(b'}') {
                        parser.punctuation(b'}');
                        depth -= 1;
                    } else {
                        states[depth] = 6;
                    }
                }
                continue;
            }
            _ => {}
        }
        states[depth] = match states[depth] {
            0 => 1,
            2 | 3 => 4,
            8 => 9,
            _ => json_error!(parser.jsonc, "invalid parser state"),
        };
        match parser.peek() {
            b'[' | b'{' => {
                let open = parser.peek();
                json_assert!(parser.jsonc, depth < 128, "nesting exceeds 128 containers");
                parser.punctuation(open);
                depth += 1;
                states[depth] = if open == b'[' { 2 } else { 5 };
            }
            b'"' => parser.string(),
            b't' => parser.literal(b"true"),
            b'f' => parser.literal(b"false"),
            b'n' => parser.literal(b"null"),
            b'-' | b'0'..=b'9' => parser.number(),
            _ => json_error!(parser.jsonc, "expected JSON value"),
        }
    }
}

pub const fn json_len(input: &str) -> usize {
    scan_json::<0>(input, false).1
}

pub(super) const fn scan_json<const N: usize>(input: &str, emit: bool) -> ([u8; N], usize) {
    scan::<N>(input, emit, false)
}

pub(super) const fn scan_jsonc<const N: usize>(input: &str, emit: bool) -> ([u8; N], usize) {
    scan::<N>(input, emit, true)
}

pub const fn jsonc_len(input: &str) -> usize {
    scan_jsonc::<0>(input, false).1
}

pub const fn jsonc<const N: usize>(input: &str) -> [u8; N] {
    let (output, written) = scan_jsonc::<N>(input, true);
    assert!(written == N);
    output
}

pub const fn json<const N: usize>(input: &str) -> [u8; N] {
    let (output, written) = scan_json::<N>(input, true);
    assert!(written == N);
    output
}
