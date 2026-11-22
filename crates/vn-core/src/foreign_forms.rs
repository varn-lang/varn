#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForeignForm {
    StrictEq,
    StrictNe,
    Var,
    Switch,
    Delete,
    IndexSignature,
    RecordType,
    ForOf,
    NumberType,
    StringType,
    BooleanType,
    ObjectType,
    SymbolType,
    AnyType,
    UnknownType,
    PromiseType,
    FunctionType,
    BoxedObjectType,
    BoxedStringType,
    BoxedNumberType,
    Undefined,
    Instanceof,
    Typeof,
    TupleBrackets,
    Overload,
    Destructor,
    ModuleBlock,
    WithStatement,
    ExportDefault,
    DefaultImport,
    MatchWithoutParens,
    ArrayHole,
    HashPrivate,
}

impl ForeignForm {
    const TYPE_NAMES: [(&'static str, Self); 12] = [
        ("number", Self::NumberType),
        ("string", Self::StringType),
        ("boolean", Self::BooleanType),
        ("object", Self::ObjectType),
        ("symbol", Self::SymbolType),
        ("any", Self::AnyType),
        ("unknown", Self::UnknownType),
        ("Promise", Self::PromiseType),
        ("Function", Self::FunctionType),
        ("Object", Self::BoxedObjectType),
        ("String", Self::BoxedStringType),
        ("Number", Self::BoxedNumberType),
    ];

    const WORDS: [(&'static str, Self); 5] = [
        ("undefined", Self::Undefined),
        ("instanceof", Self::Instanceof),
        ("typeof", Self::Typeof),
        ("destructor", Self::Destructor),
        ("module", Self::ModuleBlock),
    ];

    pub fn of_type_name(name: &str) -> Option<Self> {
        Self::TYPE_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, f)| f)
    }

    pub fn of_word(word: &str) -> Option<Self> {
        Self::WORDS
            .iter()
            .find(|(n, _)| *n == word)
            .map(|&(_, f)| f)
    }

    pub fn spelling(self) -> &'static str {
        match self {
            Self::StrictEq => "===",
            Self::StrictNe => "!==",
            Self::Var => "var",
            Self::Switch => "switch",
            Self::Delete => "delete",
            Self::IndexSignature => "[key: K]: V",
            Self::RecordType => "Record<K, V>",
            Self::ForOf => "for (... of ...)",
            Self::TupleBrackets => "[A, B]",
            Self::Overload => "function f(...): T;",
            Self::WithStatement => "with (obj) { ... }",
            Self::ExportDefault => "export default",
            Self::DefaultImport => "import name from \"module\"",
            Self::MatchWithoutParens => "match x { ... }",
            Self::ArrayHole => "[a, , b]",
            Self::HashPrivate => "#name",
            Self::NumberType
            | Self::StringType
            | Self::BooleanType
            | Self::ObjectType
            | Self::SymbolType
            | Self::AnyType
            | Self::UnknownType
            | Self::PromiseType
            | Self::FunctionType
            | Self::BoxedObjectType
            | Self::BoxedStringType
            | Self::BoxedNumberType => Self::TYPE_NAMES
                .iter()
                .find(|(_, f)| *f == self)
                .map_or("", |&(n, _)| n),
            Self::Undefined
            | Self::Instanceof
            | Self::Typeof
            | Self::Destructor
            | Self::ModuleBlock => Self::WORDS
                .iter()
                .find(|(_, f)| *f == self)
                .map_or("", |&(n, _)| n),
        }
    }

    pub fn replacement(self) -> &'static str {
        match self {
            Self::StrictEq => "`==` compares by value, write `==`",
            Self::StrictNe => "`!=` compares by value, write `!=`",
            Self::Var => "use `let` or `const`",
            Self::Switch => "write `match (x) { 1 => { ... }, _ => { ... } }`",
            Self::Delete => "remove a key with `map.delete(key)`",
            Self::IndexSignature | Self::RecordType => "write `Map<K, V>` for a keyed collection",
            Self::ForOf => "write `for (const x in xs)`",
            Self::NumberType => "write `int` or `float`, or a fixed width such as `i32` or `f64`",
            Self::StringType | Self::BoxedStringType => "write `str`",
            Self::BooleanType => "write `bool`",
            Self::ObjectType | Self::BoxedObjectType => {
                "declare the shape with an `interface`, a `struct` or a `class`"
            }
            Self::SymbolType => "use an `enum` for a closed set of keys",
            Self::AnyType | Self::UnknownType => {
                "use a union, a generic parameter, or `dynamic` at a host boundary"
            }
            Self::PromiseType => "an async result is `Task<T>`",
            Self::FunctionType => "write the signature, such as `(x: int) => str`",
            Self::BoxedNumberType => "write `int` or `float`",
            Self::Undefined => "absence is `null`, and an absent value has type `T?`",
            Self::Instanceof => "write `x is T`",
            Self::Typeof => "test the type with `x is T` or `match`",
            Self::TupleBrackets => "a tuple type is written `(A, B)`",
            Self::Overload => "use a union parameter or distinct function names",
            Self::Destructor => "implement `Disposable` and release with `using`",
            Self::ModuleBlock => "write `namespace Name { ... }`",
            Self::WithStatement => {
                "bind the values with `const`; `with` builds a copy as `value with { field: x }`"
            }
            Self::ExportDefault => {
                "export by name, as `export function f`, and import it with `import { f }`"
            }
            Self::DefaultImport => {
                "import by name, as `import { f } from \"module\"`, or the whole module with `import * as m`"
            }
            Self::MatchWithoutParens => "write the subject in parentheses: `match (x) { ... }`",
            Self::HashPrivate => "a member is private with `private name: T`, checked at compile time",
            Self::ArrayHole => "a sequence has no holes: write every element (or `null` in a `T?[]`) and discard a position with `_`",
        }
    }

    pub fn message(self) -> String {
        format!(
            "`{}` does not exist in Varn: {}",
            self.spelling(),
            self.replacement()
        )
    }
}
