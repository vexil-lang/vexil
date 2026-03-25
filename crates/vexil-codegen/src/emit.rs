/// Helper for emitting formatted Rust source code with indentation management.
pub struct CodeWriter {
    buf: String,
    indent: usize,
}

impl CodeWriter {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    /// Write a line with current indentation.
    pub fn line(&mut self, text: &str) {
        if text.is_empty() {
            self.buf.push('\n');
        } else {
            for _ in 0..self.indent {
                self.buf.push_str("    ");
            }
            self.buf.push_str(text);
            self.buf.push('\n');
        }
    }

    /// Write text without trailing newline (for partial lines).
    pub fn write(&mut self, text: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("    ");
        }
        self.buf.push_str(text);
    }

    /// Append text to current line (no indentation).
    pub fn append(&mut self, text: &str) {
        self.buf.push_str(text);
    }

    /// Increase indentation.
    pub fn indent(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation.
    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// Write an opening brace line and indent.
    pub fn open_block(&mut self, prefix: &str) {
        self.line(&format!("{prefix} {{"));
        self.indent();
    }

    /// Dedent and write closing brace.
    pub fn close_block(&mut self) {
        self.dedent();
        self.line("}");
    }

    /// Emit an empty line.
    pub fn blank(&mut self) {
        self.buf.push('\n');
    }

    /// Consume and return the built string.
    pub fn finish(self) -> String {
        self.buf
    }
}
