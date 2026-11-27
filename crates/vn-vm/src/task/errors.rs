use std::rc::Rc;

use vn_types::ClassObj;

use crate::heap::HeapInner;
use crate::value::VmValue;

pub(super) fn plain_field(heap: &HeapInner, value: VmValue) -> Option<vn_types::value::SendValue> {
    use vn_types::value::SendValue;
    if value.is_int() {
        Some(SendValue::Int(value.as_int()))
    } else if value.is_bool() {
        Some(SendValue::Bool(value.as_bool()))
    } else if heap.is_string(value) {
        Some(SendValue::Str(heap.str_repr(value)))
    } else {
        None
    }
}

pub(crate) fn derives_from_error(class: &Rc<ClassObj>) -> bool {
    let root = vn_core::RuntimeErrorKind::Error.name();
    let mut current = Rc::clone(class);
    loop {
        if current.name.as_str() == root {
            return true;
        }
        let Some(parent) = current.superclass.borrow().clone() else {
            return false;
        };
        current = parent;
    }
}

pub(crate) fn error_value(heap: &HeapInner, thrown: VmValue) -> Option<vn_types::value::SendValue> {
    use vn_core::MemberKey;
    use vn_types::value::SendValue;
    if !thrown.is_heap() {
        return None;
    }
    let carried = [
        MemberKey::Message.name(),
        MemberKey::Name.name(),
        MemberKey::Stack.name(),
    ];
    if let Some(inst) = heap.instance(thrown.as_heap())
        && let Some(class) = ClassObj::find_by_id(inst.class_id)
    {
        let layout = class.layout();
        let mut message = String::new();
        let mut fields = Vec::new();
        for field in layout.fields.iter() {
            let Some(value) = inst.read_field(field) else {
                continue;
            };
            if field.name.as_ref() == MemberKey::Message.name() {
                message = heap.str_repr(value);
            } else if !carried.contains(&field.name.as_ref())
                && let Some(plain) = plain_field(heap, value)
            {
                fields.push((field.name.to_string(), plain));
            }
        }
        return Some(SendValue::Error {
            class: class.name.to_string(),
            message,
            fields,
        });
    }
    let entries = crate::exec::value_views::object_entries(heap, thrown)?;
    let named = |name: &str| entries.iter().find(|(key, _)| key == name).map(|(_, v)| *v);
    let message = heap.str_repr(named(MemberKey::Message.name())?);
    let class = named(MemberKey::Name.name())
        .map(|v| heap.str_repr(v))
        .unwrap_or_else(|| vn_core::RuntimeErrorKind::Error.name().to_owned());
    let fields = entries
        .iter()
        .filter(|(key, _)| !carried.contains(&key.as_str()))
        .filter_map(|(key, value)| Some((key.to_string(), plain_field(heap, *value)?)))
        .collect();
    Some(SendValue::Error {
        class,
        message,
        fields,
    })
}

pub(super) fn rejection_text(heap: &HeapInner, thrown: VmValue) -> String {
    match error_value(heap, thrown) {
        Some(vn_types::value::SendValue::Error { class, message, .. }) => {
            format!("{class}: {message}")
        }
        Some(_) | None => heap.str_repr(thrown),
    }
}
