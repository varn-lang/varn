use super::*;

impl<'a> Scopes<'a> {
    pub(super) fn decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Variable(variable) => {
                for declarator in &variable.declarators {
                    if let Some(init) = declarator.init {
                        self.expr(init);
                    }
                    if let Some(ty) = &declarator.type_ann {
                        self.ty(ty);
                    }
                }
            }
            Decl::Function(function) => self.function_decl(function),
            Decl::Class(class) => self.class(class),
            Decl::Interface(interface) => self.interface(interface),
            Decl::TypeAlias(alias) => self.alias(alias),
            Decl::Enum(declaration) => self.enumeration(declaration),
            Decl::Namespace(namespace) => {
                self.stack.push(HashMap::new());
                self.hoist_decls(&namespace.body);
                for inner in &namespace.body {
                    self.decl(inner);
                }
                self.stack.pop();
            }
            Decl::Import(_) => {}
            Decl::Export(export) => match export {
                ExportDecl::Named { specifiers, .. } => {
                    for specifier in specifiers {
                        self.read(specifier.local);
                    }
                }
                ExportDecl::All { .. } => {}
                ExportDecl::Decl { declaration, .. } => self.decl(declaration),
            },
            Decl::Extension(extension) => self.extension(extension),
        }
    }
    pub(super) fn params(&mut self, params: &[Param], flag: bool, anonymous: bool) {
        let function = self.functions;
        self.functions += 1;
        for (position, param) in params.iter().enumerate() {
            let owned = param.modifiers.visibility.is_some() || param.modifiers.is_readonly;
            self.declare_pattern(
                &param.pattern,
                &Declare {
                    kind: if flag && !owned {
                        Kind::Param
                    } else {
                        Kind::Exempt
                    },
                    exported: false,
                    initialized: true,
                    group: None,
                    param_of: Some((function, position)),
                    renamable: anonymous && matches!(param.pattern, Pattern::Identifier { .. }),
                    shorthand: false,
                    import: None,
                },
            );
            if let Some(ty) = &param.type_ann {
                self.ty(ty);
            }
            if let Some(default) = param.default {
                self.expr(default);
            }
        }
    }
    pub(super) fn function_decl(&mut self, function: &FunctionDecl) {
        self.decorators(&function.decorators);
        self.stack.push(HashMap::new());
        self.type_params(&function.type_params);
        self.params(&function.params, !function.modifiers.is_declare, false);
        if let Some(ty) = &function.return_type {
            self.ty(ty);
        }
        self.stmt(function.body);
        self.stack.pop();
    }
    pub(super) fn decorators(&mut self, list: &[Decorator]) {
        for decorator in list {
            self.expr(decorator.expression);
        }
    }
    pub(super) fn type_params(&mut self, list: &[TypeParam]) {
        for param in list {
            if let Some(ty) = &param.constraint {
                self.ty(ty);
            }
            if let Some(ty) = &param.default {
                self.ty(ty);
            }
        }
    }
    pub(super) fn class(&mut self, class: &ClassDecl) {
        self.decorators(&class.decorators);
        self.type_params(&class.type_params);
        if let Some(parent) = class.super_class {
            self.expr(parent);
        }
        for ty in class.super_type_args.iter().chain(&class.implements) {
            self.ty(ty);
        }
        for member in &class.body {
            self.member(member);
        }
    }
    pub(super) fn member(&mut self, member: &ClassMember) {
        match member {
            ClassMember::Constructor {
                params,
                body,
                decorators,
                ..
            } => {
                self.decorators(decorators);
                self.stack.push(HashMap::new());
                self.params(params, false, false);
                self.stmt(*body);
                self.stack.pop();
            }
            ClassMember::Method {
                type_params,
                params,
                return_type,
                body,
                decorators,
                ..
            } => {
                self.decorators(decorators);
                self.stack.push(HashMap::new());
                self.type_params(type_params);
                self.params(params, false, false);
                if let Some(ty) = return_type {
                    self.ty(ty);
                }
                if let Some(body) = body {
                    self.stmt(*body);
                }
                self.stack.pop();
            }
            ClassMember::Property {
                type_ann,
                init,
                decorators,
                ..
            } => {
                self.decorators(decorators);
                if let Some(ty) = type_ann {
                    self.ty(ty);
                }
                if let Some(init) = init {
                    self.expr(*init);
                }
            }
            ClassMember::Getter {
                return_type,
                body,
                decorators,
                ..
            } => {
                self.decorators(decorators);
                if let Some(ty) = return_type {
                    self.ty(ty);
                }
                if let Some(body) = body {
                    self.stmt(*body);
                }
            }
            ClassMember::Setter {
                param,
                body,
                decorators,
                ..
            } => {
                self.decorators(decorators);
                self.stack.push(HashMap::new());
                self.params(std::slice::from_ref(param), false, false);
                if let Some(body) = body {
                    self.stmt(*body);
                }
                self.stack.pop();
            }
            ClassMember::StaticBlock { body, .. } => self.stmt(*body),
        }
    }
    pub(super) fn interface(&mut self, interface: &InterfaceDecl) {
        self.type_params(&interface.type_params);
        for ty in &interface.extends {
            self.ty(ty);
        }
        for member in &interface.body {
            self.interface_member(member);
        }
    }
    pub(super) fn interface_member(&mut self, member: &InterfaceMember) {
        match member {
            InterfaceMember::Property { type_ann, .. } => self.ty(type_ann),
            InterfaceMember::Method {
                type_params,
                params,
                return_type,
                ..
            } => {
                self.type_params(type_params);
                for param in params {
                    if let Some(ty) = &param.type_ann {
                        self.ty(ty);
                    }
                }
                if let Some(ty) = return_type {
                    self.ty(ty);
                }
            }
            InterfaceMember::Callable {
                params,
                return_type,
                ..
            } => {
                for param in params {
                    if let Some(ty) = &param.type_ann {
                        self.ty(ty);
                    }
                }
                self.ty(return_type);
            }
        }
    }
    pub(super) fn alias(&mut self, alias: &TypeAliasDecl) {
        self.type_params(&alias.type_params);
        self.ty(&alias.alias);
    }
    pub(super) fn enumeration(&mut self, declaration: &EnumDecl) {
        self.type_params(&declaration.type_params);
        for ty in &declaration.implements {
            self.ty(ty);
        }
        for member in &declaration.members {
            if let Some(init) = member.init {
                self.expr(init);
            }
            for field in &member.payload_fields {
                self.ty(&field.ty);
                if let Some(init) = field.init {
                    self.expr(init);
                }
            }
        }
        for member in &declaration.body {
            self.member(member);
        }
    }
    pub(super) fn extension(&mut self, extension: &ExtensionDecl) {
        self.type_params(&extension.type_params);
        self.ty(&extension.target);
        for member in &extension.members {
            match member {
                ExtensionMember::Method(function) => self.function_decl(function),
                ExtensionMember::Getter {
                    return_type, body, ..
                } => {
                    if let Some(ty) = return_type {
                        self.ty(ty);
                    }
                    self.stmt(*body);
                }
                ExtensionMember::Setter { param, body, .. } => {
                    self.stack.push(HashMap::new());
                    self.params(std::slice::from_ref(param), false, false);
                    self.stmt(*body);
                    self.stack.pop();
                }
            }
        }
    }
}
