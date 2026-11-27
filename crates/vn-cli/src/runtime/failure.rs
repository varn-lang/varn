use vn_pipeline::{RuntimeFailure, StackFrame};

pub fn runtime_failure(error: vn_vm::RuntimeError) -> RuntimeFailure {
    RuntimeFailure {
        class: error.class,
        message: error.message,
        frames: error
            .frames
            .into_iter()
            .map(|frame| StackFrame {
                fn_name: frame.fn_name,
                file: frame.file,
                line: frame.line,
            })
            .collect(),
        cause: error.cause.map(|inner| Box::new(runtime_failure(*inner))),
    }
}
