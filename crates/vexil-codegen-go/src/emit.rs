/// Helper for emitting formatted Go source code with tab-based indentation.
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
                self.buf.push('\t');
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

    /// Write an opening brace line and indent.
    pub fn open_block(&mut self, prefix: &str) {
        if prefix.is_empty() {
            self.line("{");
        } else {
            self.line(&format!("{prefix} {{"));
        }
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
