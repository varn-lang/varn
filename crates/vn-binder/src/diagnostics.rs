use vn_core::Diagnostic;

impl super::Binder<'_> {
    pub(super) fn emit(&mut self, diag: Diagnostic) {
        let diag = if diag.file.is_empty() {
            diag.with_file(self.source_file.clone())
        } else {
            diag
        };
        self.diagnostics.push(diag);
    }

    pub(super) fn require_task_signature(
        &mut self,
        declared: vn_sem::types::Type,
        annotation: Option<&vn_core::ast::TypeNode>,
        is_async: bool,
        is_generator: bool,
    ) {
        let Some(annotation) = annotation else {
            return;
        };
        if !is_async
            || is_generator
            || declared.is_unresolved()
            || vn_sem::types::is_awaitable(&declared, &self.ty_table)
        {
            return;
        }
        let shown = declared.display(&self.ty_table, &self.interner);
        self.emit(
            Diagnostic::error(
                vn_core::ErrorCode::AsyncReturnNotTask,
                format!("an `async` function returns a task: write `Task<{shown}>`"),
            )
            .with_range(annotation.range),
        );
    }
}
