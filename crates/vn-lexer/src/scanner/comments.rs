impl super::Scanner<'_> {
    pub(super) fn skip_line_comment(&mut self) {
        while !self.is_eof() && self.peek(0) != b'\n' {
            self.cur_col += 1;
            self.pos += 1;
        }
    }

    pub(super) fn skip_block_comment(&mut self) {
        let start = self.pos - 2;
        while !self.is_eof() {
            if self.peek(0) == b'*' && self.peek(1) == b'/' {
                self.cur_col += 2;
                self.pos += 2;
                return;
            }
            self.advance_byte();
        }
        self.report_unterminated(
            vn_core::ErrorCode::UnterminatedBlockComment,
            "unterminated block comment",
            start,
        );
    }

    pub(super) fn scan_doc_comment(&mut self, comment_start: usize) {
        self.cur_col += 1;
        self.pos += 1;

        let content_start = self.pos;

        while !self.is_eof() {
            if self.peek(0) == b'*' && self.peek(1) == b'/' {
                break;
            }
            self.advance_byte();
        }
        let content_end = self.pos;
        if self.is_eof() {
            self.report_unterminated(
                vn_core::ErrorCode::UnterminatedBlockComment,
                "unterminated block comment",
                comment_start,
            );
        } else {
            self.cur_col += 2;
            self.pos += 2;
        }

        let text: std::sync::Arc<str> =
            String::from_utf8_lossy(&self.src[content_start..content_end]).into();
        self.push_trivia(vn_core::TriviaKind::Doc(text), comment_start);
    }
}
