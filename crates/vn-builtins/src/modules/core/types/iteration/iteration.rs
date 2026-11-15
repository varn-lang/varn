use vn_macros::natives;
use vn_types::{NativeCtx, NativeError, VmValue};


fn resume(ctx: &mut dyn NativeCtx, this: VmValue) -> Result<VmValue, NativeError> {
    ctx.resume_generator(this, false)
}

natives! {
    class Generator {
        next(ctx, this) => Some(resume(ctx, this)?).filter(|value| !value.is_null()),
        close(ctx, this) => ctx.close_generator(this),
    }
    class AsyncGenerator {
        next(ctx, this) => {
            match resume(ctx, this) {
                Ok(task) => task,
                Err(error) => {
                    let thrown = match error.thrown {
                        Some(value) => value,
                        None => ctx.alloc_str_owned(error.message),
                    };
                    ctx.task_rejected(thrown)
                }
            }
        },
        close(ctx, this) => ctx.close_async_generator(this),
    }
}
