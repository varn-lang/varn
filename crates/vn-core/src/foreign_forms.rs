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
    ParseNumber,
    JsonGlobal,
    MathGlobal,
    IsNaN,
    BoxingCall,
    HostGlobal,
    TimerGlobal,
    SymbolGlobal,
    Require,
    DateGlobal,
    ObjectGlobal,
    KeyofType,
    IndexedAccessType,
    MappedType,
    ConditionalType,
    InferType,
    TemplateLiteralType,
    OptionalityUtility,
    ReadonlyUtility,
    ShapeUtility,
    UnionUtility,
    NonNullableUtility,
    ReturnTypeUtility,
    AwaitedUtility,
}

impl ForeignForm {
    const TYPE_NAMES: [(&'static str, Self); 23] = [
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
        ("Partial", Self::OptionalityUtility),
        ("Required", Self::OptionalityUtility),
        ("Mutable", Self::OptionalityUtility),
        ("Readonly", Self::ReadonlyUtility),
        ("Pick", Self::ShapeUtility),
        ("Omit", Self::ShapeUtility),
        ("Exclude", Self::UnionUtility),
        ("Extract", Self::UnionUtility),
        ("NonNullable", Self::NonNullableUtility),
        ("ReturnType", Self::ReturnTypeUtility),
        ("Awaited", Self::AwaitedUtility),
    ];

    const WORDS: [(&'static str, Self); 23] = [
        ("undefined", Self::Undefined),
        ("instanceof", Self::Instanceof),
        ("typeof", Self::Typeof),
        ("destructor", Self::Destructor),
        ("module", Self::ModuleBlock),
        ("parseInt", Self::ParseNumber),
        ("parseFloat", Self::ParseNumber),
        ("JSON", Self::JsonGlobal),
        ("Math", Self::MathGlobal),
        ("isNaN", Self::IsNaN),
        ("Boolean", Self::BoxingCall),
        ("globalThis", Self::HostGlobal),
        ("window", Self::HostGlobal),
        ("process", Self::HostGlobal),
        ("setTimeout", Self::TimerGlobal),
        ("setInterval", Self::TimerGlobal),
        ("Symbol", Self::SymbolGlobal),
        ("require", Self::Require),
        ("Date", Self::DateGlobal),
        ("Object", Self::ObjectGlobal),
        ("Array", Self::BoxingCall),
        ("Number", Self::BoxingCall),
        ("String", Self::BoxingCall),
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
            Self::KeyofType => "keyof T",
            Self::IndexedAccessType => "T[K]",
            Self::MappedType => "{ [K in T]: V }",
            Self::ConditionalType => "A extends B ? X : Y",
            Self::InferType => "infer T",
            Self::TemplateLiteralType => "`prefix${T}`",
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
            | Self::BoxedNumberType
            | Self::OptionalityUtility
            | Self::ReadonlyUtility
            | Self::ShapeUtility
            | Self::UnionUtility
            | Self::NonNullableUtility
            | Self::ReturnTypeUtility
            | Self::AwaitedUtility => Self::TYPE_NAMES
                .iter()
                .find(|(_, f)| *f == self)
                .map_or("", |&(n, _)| n),
            Self::Undefined
            | Self::Instanceof
            | Self::Typeof
            | Self::Destructor
            | Self::ModuleBlock
            | Self::ParseNumber
            | Self::JsonGlobal
            | Self::MathGlobal
            | Self::IsNaN
            | Self::BoxingCall
            | Self::HostGlobal
            | Self::TimerGlobal
            | Self::SymbolGlobal
            | Self::Require
            | Self::DateGlobal
            | Self::ObjectGlobal => Self::WORDS
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
            Self::TupleBrackets => "a tuple type is written `#[A, B]`",
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
            Self::ParseNumber => "parse with `int.parse(s)` or `float.parse(s)`, which reject trailing text",
            Self::JsonGlobal => "import `Json` from \"std:encoding\" and decode into a declared type",
            Self::MathGlobal => "import the functions from \"std:math\", such as `import { max } from \"std:math\"`",
            Self::IsNaN => "ask the value: `x.isNaN()`",
            Self::BoxingCall => "there are no boxing constructors: convert with `x.toString()`, `int.parse(s)` or a comparison",
            Self::HostGlobal => "there is no global object: import what you need from \"std:sys\" or \"std:process\"",
            Self::TimerGlobal => "wait with `await sleep(ms)` from \"std:task\"",
            Self::SymbolGlobal => "use an `enum` for a closed set of keys",
            Self::Require => "import the module with `import { name } from \"module\"`",
            Self::DateGlobal => "read the clock with `now()` from \"std:time\"",
            Self::ObjectGlobal => "a record has the fields of its declared type; keyed data is a `Map<K, V>`",
            Self::ArrayHole => "a sequence has no holes: write every element (or `null` in a `T?[]`) and discard a position with `_`",
            Self::KeyofType => "a type has no set of keys to compute with: name a closed set of keys with an `enum`, or keep keyed data in a `Map<K, V>`",
            Self::IndexedAccessType => "write the member's type itself, or give it a name with `type`",
            Self::MappedType | Self::ShapeUtility => "declare the shape you need as its own `interface` or `struct`",
            Self::ConditionalType | Self::InferType => "types are not computed: declare the operation only where it applies, as `extension<U> on U[][]`, and write its result in the signature",
            Self::TemplateLiteralType => "a set of allowed texts is a union of literals or an `enum`",
            Self::OptionalityUtility => "a value is built against its declared type, a field that may be absent is declared `x?: T`, and a changed copy is `value with { field: x }`",
            Self::ReadonlyUtility => "a read-only view is `readonly T[]` or `ReadonlyMap<K, V>`, and a fixed field is declared `readonly`",
            Self::UnionUtility => "write the union you mean, or take it apart with `match`",
            Self::NonNullableUtility => "write the type without `?`, or narrow the value with `x != null`",
            Self::ReturnTypeUtility => "write the result type in the signature",
            Self::AwaitedUtility => "`await` on a `Task<T>` gives a `T`: write `T`",
        }
    }

    pub fn message(self) -> String {
        self.message_as(self.spelling())
    }

    pub fn message_as(self, written: &str) -> String {
        format!("`{written}` does not exist in Varn: {}", self.replacement())
    }
}
