use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn ty(&mut self, node: &TypeNode) {
        match &node.kind {
            TypeKind::Primitive(_) | TypeKind::Literal(_) | TypeKind::This => {}
            TypeKind::Array(inner) => self.ty(inner),
            TypeKind::Union(list) | TypeKind::Intersection(list) | TypeKind::Tuple(list) => {
                for item in list {
                    self.ty(item);
                }
            }
            TypeKind::Named(name, ()) => self.read_qualified(*name),
            TypeKind::Generic(name, args, ()) => {
                self.read_qualified(*name);
                for arg in args {
                    self.ty(arg);
                }
            }
            TypeKind::Fn((params, ret)) => {
                for param in params {
                    self.ty(&param.ty);
                }
                self.ty(ret);
            }
            TypeKind::Record(members) => {
                for member in members {
                    self.interface_member(member);
                }
            }
            TypeKind::EnumVariant {
                enum_name,
                type_args,
                payload_ty,
                ..
            } => {
                self.read_qualified(*enum_name);
                for arg in type_args {
                    self.ty(arg);
                }
                self.ty(payload_ty);
            }
            TypeKind::TypePredicate {
                parameter_name,
                target_type,
            } => {
                self.read_qualified(*parameter_name);
                self.ty(target_type);
            }
            TypeKind::Interface(never) => match *never {},
        }
    }
}
