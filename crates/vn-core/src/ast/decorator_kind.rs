use super::decorators::BuiltinDecorator;

crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BuiltinDecoratorKind {
        Deprecated => "deprecated",
        Pure => "pure",
        Inline => "inline",
        Capability => "capability",
        Test => "test",
        Throws => "throws",
        Decodes => "decodes",
    }
}

impl BuiltinDecoratorKind {
    pub const fn supports(self, target: DecoratorTarget) -> bool {
        match self {
            Self::Deprecated => true,
            Self::Pure => matches!(
                target,
                DecoratorTarget::Function
                    | DecoratorTarget::Method
                    | DecoratorTarget::Getter
                    | DecoratorTarget::Setter
            ),
            Self::Capability => matches!(
                target,
                DecoratorTarget::Function
                    | DecoratorTarget::Method
                    | DecoratorTarget::Getter
                    | DecoratorTarget::Setter
                    | DecoratorTarget::Constructor
            ),
            Self::Throws => matches!(
                target,
                DecoratorTarget::Function
                    | DecoratorTarget::Method
                    | DecoratorTarget::Getter
                    | DecoratorTarget::Setter
                    | DecoratorTarget::Constructor
            ),
            Self::Inline | Self::Test | Self::Decodes => {
                matches!(target, DecoratorTarget::Function)
            }
        }
    }
}

impl BuiltinDecorator {
    pub fn kind(&self) -> BuiltinDecoratorKind {
        match self {
            Self::Deprecated { .. } => BuiltinDecoratorKind::Deprecated,
            Self::Pure => BuiltinDecoratorKind::Pure,
            Self::Inline => BuiltinDecoratorKind::Inline,
            Self::Capability { .. } => BuiltinDecoratorKind::Capability,
            Self::Test => BuiltinDecoratorKind::Test,
            Self::Throws { .. } => BuiltinDecoratorKind::Throws,
            Self::Decodes { .. } => BuiltinDecoratorKind::Decodes,
        }
    }
}

crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DecoratorTarget {
        Function => crate::TokenKind::Function.as_str(),
        Method => "method",
        Class => crate::TokenKind::Class.as_str(),
        Getter => "getter",
        Setter => "setter",
        Constructor => crate::TokenKind::Constructor.as_str(),
        Property => "property",
    }
}

impl DecoratorTarget {
    pub const fn arity(self) -> usize {
        match self {
            Self::Function | Self::Class => 1,
            Self::Method | Self::Getter | Self::Setter | Self::Constructor | Self::Property => 2,
        }
    }

    pub const fn returns_value(self) -> bool {
        !matches!(self, Self::Property)
    }
}
