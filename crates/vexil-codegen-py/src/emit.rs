/// Helper for emitting formatted Python source code with indentation.
pub struct CodeWriter {
    buf: String,
    indent: usize,
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::new()
    }
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

    /// Increase indentation.
    pub fn indent(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation.
    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    /// Write a def/class/if/for line and indent.
    pub fn open_block(&mut self, header: &str) {
        self.line(&format!("{header}:"));
        self.indent();
    }

    /// Dedent (closes a block — Python uses no braces).
    pub fn close_block(&mut self) {
        self.dedent();
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
