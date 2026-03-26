use crate::error::VxError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `@schema`, `@version`, etc.
    Directive(String),
    /// Identifier: type name, field name, keyword
    Ident(String),
    /// Quoted string literal
    StringLit(String),
    /// Integer literal
    IntLit(i128),
    /// Float literal
    FloatLit(f64),
    /// Hex byte sequence: `0x[01 02 ab]`
    HexBytes(Vec<u8>),
    /// Base64 byte sequence: `b64"..."`
    Base64Bytes(Vec<u8>),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Comma,
    Pipe,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    col: usize,
    pub file: String,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str, file: impl Into<String>) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            line: 1,
            col: 1,
            file: file.into(),
        }
    }

    /// Lex all tokens from the input, skipping comments.
    pub fn lex_all(&mut self) -> Result<Vec<Spanned>, VxError> {
        let mut tokens = Vec::new();
        loop {
            let spanned = self.next_token()?;
            let is_eof = spanned.token == Token::Eof;
            tokens.push(spanned);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_char(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((_, c)) = result {
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        result
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn current_span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek_char() {
            self.next_char();
            if c == '\n' {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Spanned, VxError> {
        loop {
            self.skip_whitespace();
            let span = self.current_span();

            match self.peek_char() {
                None => {
                    return Ok(Spanned {
                        token: Token::Eof,
                        span,
                    })
                }
                Some('#') => {
                    self.skip_line_comment();
                    continue;
                }
                Some('/') => {
                    // Could be // comment
                    self.next_char();
                    if self.peek_char() == Some('/') {
                        self.skip_line_comment();
                        continue;
                    } else {
                        return Err(VxError::Parse {
                            file: self.file.clone(),
                            line: span.line,
                            col: span.col,
                            message: "unexpected '/'".to_string(),
                        });
                    }
                }
                Some('{') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::LBrace,
                        span,
                    });
                }
                Some('}') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::RBrace,
                        span,
                    });
                }
                Some('[') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::LBracket,
                        span,
                    });
                }
                Some(']') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::RBracket,
                        span,
                    });
                }
                Some('(') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::LParen,
                        span,
                    });
                }
                Some(')') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::RParen,
                        span,
                    });
                }
                Some(':') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::Colon,
                        span,
                    });
                }
                Some(',') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::Comma,
                        span,
                    });
                }
                Some('|') => {
                    self.next_char();
                    return Ok(Spanned {
                        token: Token::Pipe,
                        span,
                    });
                }
                Some('@') => {
                    self.next_char();
                    let name = self.read_ident();
                    return Ok(Spanned {
                        token: Token::Directive(name),
                        span,
                    });
                }
                Some('"') => {
                    let s = self.read_string(span)?;
                    return Ok(Spanned {
                        token: Token::StringLit(s),
                        span,
                    });
                }
                Some('0') => {
                    return self.read_zero_prefix(span);
                }
                Some('-') => {
                    self.next_char();
                    // Could be negative number or `-Inf`
                    if self
                        .peek_char()
                        .map(|c| c.is_ascii_alphabetic())
                        .unwrap_or(false)
                    {
                        let word = self.read_ident();
                        if word == "Inf" {
                            return Ok(Spanned {
                                token: Token::FloatLit(f64::NEG_INFINITY),
                                span,
                            });
                        }
                        return Err(VxError::Parse {
                            file: self.file.clone(),
                            line: span.line,
                            col: span.col,
                            message: format!("unexpected identifier after '-': {word}"),
                        });
                    }
                    return self.read_number_after_sign(span, true);
                }
                Some(c) if c.is_ascii_digit() => {
                    return self.read_number_after_sign(span, false);
                }
                Some(c) if c.is_alphabetic() || c == '_' => {
                    // Could be: NaN, Inf, b64"...", or plain ident
                    let word = self.read_ident();
                    match word.as_str() {
                        "NaN" => {
                            return Ok(Spanned {
                                token: Token::FloatLit(f64::NAN),
                                span,
                            })
                        }
                        "Inf" => {
                            return Ok(Spanned {
                                token: Token::FloatLit(f64::INFINITY),
                                span,
                            })
                        }
                        "b64" => {
                            // Base64 bytes: b64"..."
                            if self.peek_char() == Some('"') {
                                let s = self.read_string(span)?;
                                let bytes = base64_decode(&s).map_err(|e| VxError::Parse {
                                    file: self.file.clone(),
                                    line: span.line,
                                    col: span.col,
                                    message: format!("invalid base64: {e}"),
                                })?;
                                return Ok(Spanned {
                                    token: Token::Base64Bytes(bytes),
                                    span,
                                });
                            }
                            return Ok(Spanned {
                                token: Token::Ident(word),
                                span,
                            });
                        }
                        _ => {
                            return Ok(Spanned {
                                token: Token::Ident(word),
                                span,
                            })
                        }
                    }
                }
                Some(c) => {
                    self.next_char();
                    return Err(VxError::Parse {
                        file: self.file.clone(),
                        line: span.line,
                        col: span.col,
                        message: format!("unexpected character: {c:?}"),
                    });
                }
            }
        }
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                s.push(c);
                self.next_char();
            } else {
                break;
            }
        }
        s
    }

    fn read_string(&mut self, span: Span) -> Result<String, VxError> {
        self.next_char(); // consume opening '"'
        let mut s = String::new();
        loop {
            match self.next_char() {
                None => {
                    return Err(VxError::Parse {
                        file: self.file.clone(),
                        line: span.line,
                        col: span.col,
                        message: "unterminated string literal".to_string(),
                    })
                }
                Some((_, '"')) => break,
                Some((_, '\\')) => match self.next_char() {
                    Some((_, 'n')) => s.push('\n'),
                    Some((_, 't')) => s.push('\t'),
                    Some((_, 'r')) => s.push('\r'),
                    Some((_, '"')) => s.push('"'),
                    Some((_, '\\')) => s.push('\\'),
                    Some((_, '0')) => s.push('\0'),
                    Some((_, c)) => {
                        return Err(VxError::Parse {
                            file: self.file.clone(),
                            line: self.line,
                            col: self.col,
                            message: format!("unknown escape: \\{c}"),
                        });
                    }
                    None => {
                        return Err(VxError::Parse {
                            file: self.file.clone(),
                            line: span.line,
                            col: span.col,
                            message: "unexpected end after backslash".to_string(),
                        })
                    }
                },
                Some((_, c)) => s.push(c),
            }
        }
        Ok(s)
    }

    fn read_zero_prefix(&mut self, span: Span) -> Result<Spanned, VxError> {
        self.next_char(); // consume '0'
        match self.peek_char() {
            Some('x') | Some('X') => {
                self.next_char(); // consume 'x'
                                  // Check if it's 0x[...] (hex bytes) or 0xFF (hex integer)
                if self.peek_char() == Some('[') {
                    self.next_char(); // consume '['
                    let bytes = self.read_hex_byte_array(span)?;
                    return Ok(Spanned {
                        token: Token::HexBytes(bytes),
                        span,
                    });
                }
                // Hex integer
                let mut hex = String::new();
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_hexdigit() || c == '_' {
                        if c != '_' {
                            hex.push(c);
                        }
                        self.next_char();
                    } else {
                        break;
                    }
                }
                let value = i128::from_str_radix(&hex, 16).map_err(|e| VxError::Parse {
                    file: self.file.clone(),
                    line: span.line,
                    col: span.col,
                    message: format!("invalid hex integer: {e}"),
                })?;
                Ok(Spanned {
                    token: Token::IntLit(value),
                    span,
                })
            }
            Some(c) if c.is_ascii_digit() => {
                // Decimal starting with 0
                self.read_number_after_sign(span, false)
            }
            Some('.') => {
                // 0.xxx float
                self.read_float_after_digits("0", span)
            }
            _ => {
                // Just "0"
                Ok(Spanned {
                    token: Token::IntLit(0),
                    span,
                })
            }
        }
    }

    fn read_hex_byte_array(&mut self, span: Span) -> Result<Vec<u8>, VxError> {
        let mut bytes = Vec::new();
        loop {
            // Skip whitespace
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() {
                    self.next_char();
                } else {
                    break;
                }
            }
            match self.peek_char() {
                Some(']') => {
                    self.next_char();
                    break;
                }
                Some(c) if c.is_ascii_hexdigit() => {
                    let hi = c;
                    self.next_char();
                    let lo = self.peek_char().ok_or_else(|| VxError::Parse {
                        file: self.file.clone(),
                        line: span.line,
                        col: span.col,
                        message: "expected second hex digit in byte array".to_string(),
                    })?;
                    if !lo.is_ascii_hexdigit() {
                        return Err(VxError::Parse {
                            file: self.file.clone(),
                            line: span.line,
                            col: span.col,
                            message: format!("expected hex digit, got: {lo:?}"),
                        });
                    }
                    self.next_char();
                    let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).map_err(|e| {
                        VxError::Parse {
                            file: self.file.clone(),
                            line: span.line,
                            col: span.col,
                            message: format!("invalid hex byte: {e}"),
                        }
                    })?;
                    bytes.push(byte);
                }
                Some(c) => {
                    return Err(VxError::Parse {
                        file: self.file.clone(),
                        line: span.line,
                        col: span.col,
                        message: format!("unexpected character in hex byte array: {c:?}"),
                    });
                }
                None => {
                    return Err(VxError::Parse {
                        file: self.file.clone(),
                        line: span.line,
                        col: span.col,
                        message: "unterminated hex byte array".to_string(),
                    });
                }
            }
        }
        Ok(bytes)
    }

    fn read_number_after_sign(&mut self, span: Span, negative: bool) -> Result<Spanned, VxError> {
        let mut digits = String::new();
        if negative {
            digits.push('-');
        }

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    digits.push(c);
                }
                self.next_char();
            } else {
                break;
            }
        }

        // Check for float
        if self.peek_char() == Some('.')
            || self.peek_char() == Some('e')
            || self.peek_char() == Some('E')
        {
            return self.read_float_after_digits(&digits, span);
        }

        let value: i128 = digits.parse().map_err(|e| VxError::Parse {
            file: self.file.clone(),
            line: span.line,
            col: span.col,
            message: format!("invalid integer: {e}"),
        })?;
        Ok(Spanned {
            token: Token::IntLit(value),
            span,
        })
    }

    fn read_float_after_digits(&mut self, int_part: &str, span: Span) -> Result<Spanned, VxError> {
        let mut s = int_part.to_string();
        if self.peek_char() == Some('.') {
            s.push('.');
            self.next_char();
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.next_char();
                } else {
                    break;
                }
            }
        }
        if self.peek_char() == Some('e') || self.peek_char() == Some('E') {
            s.push('e');
            self.next_char();
            if let Some(c @ ('+' | '-')) = self.peek_char() {
                s.push(c);
                self.next_char();
            }
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.next_char();
                } else {
                    break;
                }
            }
        }
        let value: f64 = s.parse().map_err(|e| VxError::Parse {
            file: self.file.clone(),
            line: span.line,
            col: span.col,
            message: format!("invalid float: {e}"),
        })?;
        Ok(Spanned {
            token: Token::FloatLit(value),
            span,
        })
    }
}

/// Minimal base64 decoder (RFC 4648, no padding required).
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [255u8; 128];
    for (i, &c) in alphabet.iter().enumerate() {
        table[c as usize] = i as u8;
    }

    let bytes: Vec<u8> = s
        .bytes()
        .filter(|&b| b != b'=' && b != b'\n' && b != b'\r' && b != b' ')
        .collect();

    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = table.get(bytes[i] as usize).copied().unwrap_or(255);
        let b = table.get(bytes[i + 1] as usize).copied().unwrap_or(255);
        let c = table.get(bytes[i + 2] as usize).copied().unwrap_or(255);
        let d = table.get(bytes[i + 3] as usize).copied().unwrap_or(255);
        if a == 255 || b == 255 || c == 255 || d == 255 {
            return Err(format!("invalid base64 character at position {i}"));
        }
        out.push((a << 2) | (b >> 4));
        out.push((b << 4) | (c >> 2));
        out.push((c << 6) | d);
        i += 4;
    }
    match bytes.len() - i {
        0 => {}
        2 => {
            let a = table.get(bytes[i] as usize).copied().unwrap_or(255);
            let b = table.get(bytes[i + 1] as usize).copied().unwrap_or(255);
            if a == 255 || b == 255 {
                return Err("invalid base64 tail".to_string());
            }
            out.push((a << 2) | (b >> 4));
        }
        3 => {
            let a = table.get(bytes[i] as usize).copied().unwrap_or(255);
            let b = table.get(bytes[i + 1] as usize).copied().unwrap_or(255);
            let c = table.get(bytes[i + 2] as usize).copied().unwrap_or(255);
            if a == 255 || b == 255 || c == 255 {
                return Err("invalid base64 tail".to_string());
            }
            out.push((a << 2) | (b >> 4));
            out.push((b << 4) | (c >> 2));
        }
        _ => return Err("invalid base64 length".to_string()),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(src: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(src, "test");
        lexer
            .lex_all()
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn lex_directive() {
        let toks = tokens("@schema \"test.simple\"");
        assert!(matches!(&toks[0], Token::Directive(s) if s == "schema"));
        assert!(matches!(&toks[1], Token::StringLit(s) if s == "test.simple"));
    }

    #[test]
    fn lex_message() {
        let toks = tokens("Foo { x: 42 }");
        assert!(matches!(&toks[0], Token::Ident(s) if s == "Foo"));
        assert_eq!(toks[1], Token::LBrace);
        assert!(matches!(&toks[2], Token::Ident(s) if s == "x"));
        assert_eq!(toks[3], Token::Colon);
        assert!(matches!(toks[4], Token::IntLit(42)));
        assert_eq!(toks[5], Token::RBrace);
    }

    #[test]
    fn lex_string_with_escapes() {
        let toks = tokens(r#""hello\nworld""#);
        assert!(matches!(&toks[0], Token::StringLit(s) if s == "hello\nworld"));
    }

    #[test]
    fn lex_hex_bytes() {
        let toks = tokens("0x[de ad be ef]");
        assert!(matches!(&toks[0], Token::HexBytes(b) if b == &[0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn lex_float() {
        let toks = tokens("3.14");
        assert!(matches!(toks[0], Token::FloatLit(f) if (f - 3.14).abs() < 1e-10));
    }

    #[test]
    fn lex_flags_pipe() {
        let toks = tokens("Read | Write | Exec");
        assert!(matches!(&toks[0], Token::Ident(s) if s == "Read"));
        assert_eq!(toks[1], Token::Pipe);
        assert!(matches!(&toks[2], Token::Ident(s) if s == "Write"));
    }

    #[test]
    fn lex_comments_skipped() {
        let toks = tokens("# comment\n// also comment\n42");
        assert!(matches!(toks[0], Token::IntLit(42)));
    }

    #[test]
    fn lex_hex_integer() {
        let toks = tokens("0xFF");
        assert!(matches!(toks[0], Token::IntLit(255)));
    }

    #[test]
    fn lex_negative_integer() {
        let toks = tokens("-42");
        assert!(matches!(toks[0], Token::IntLit(-42)));
    }
}
