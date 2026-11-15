use vn_core::{RuntimeErrorKind, StdErrorClass};
use vn_macros::natives;
use vn_types::{NativeError, VmValue};


fn range_error(message: String) -> NativeError {
    NativeError::rethrow(RuntimeErrorKind::RangeError, message, None)
}

natives! {
    errorCode(className) => {
        StdErrorClass::of_name(className)
            .map(|class| class.code().to_owned())
            .ok_or_else(|| range_error(format!("'{className}' is not a std error class")))
    },
    registerErrorClasses(ctx, classes) => {
        let mut seen = Vec::new();
        for index in 0..classes.len(ctx) {
            let value: VmValue = classes.get(ctx, index).unwrap_or_else(VmValue::null);
            let cls = ctx
                .class_from_value(value)
                .ok_or_else(|| range_error(format!("registerErrorClasses: item {index} is not a class")))?;
            let class = StdErrorClass::of_name(&cls.name).ok_or_else(|| {
                range_error(format!("'{}' is not in the std error table", cls.name))
            })?;
            let parent = cls.superclass.borrow().as_ref().map(|p| p.name.clone());
            let expected = class
                .parent()
                .map_or(RuntimeErrorKind::Error.class_name(), StdErrorClass::name);
            if parent.as_deref() != Some(expected) {
                return Err(range_error(format!(
                    "'{}' must extend '{expected}', it extends '{}'",
                    cls.name,
                    parent.as_deref().unwrap_or("nothing")
                )));
            }
            ctx.register_class(class.name(), cls);
            seen.push(class);
        }
        for class in StdErrorClass::ALL {
            if !seen.contains(&class) {
                return Err(range_error(format!(
                    "registerErrorClasses: '{}' is missing",
                    class.name()
                )));
            }
        }
        Ok(())
    },
}
