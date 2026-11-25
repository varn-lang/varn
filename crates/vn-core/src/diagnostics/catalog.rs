crate::named_enum! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(u32)]
    pub enum ErrorCode {
        UnterminatedString = 1001 => "unterminated-string",
        InvalidNumericLiteral = 1002 => "invalid-numeric-literal",
        UnknownCharacter = 1003 => "unknown-character",
        UnterminatedTemplate = 1004 => "unterminated-template",
        UnterminatedRegex = 1005 => "unterminated-regex",
        InvalidEscapeSequence = 1006 => "invalid-escape-sequence",
        UnterminatedBlockComment = 1007 => "unterminated-block-comment",
        InvalidUnicodeEscape = 1008 => "invalid-unicode-escape",
        ExpectedToken = 2001 => "expected-token",
        UnexpectedToken = 2002 => "unexpected-token",
        UnterminatedParen = 2003 => "unterminated-paren",
        UnterminatedBrace = 2004 => "unterminated-brace",
        UnterminatedBracket = 2005 => "unterminated-bracket",
        InvalidExpression = 2006 => "invalid-expression",
        InvalidStatement = 2007 => "invalid-statement",
        InvalidTypeAnnotation = 2008 => "invalid-type-annotation",
        InvalidImportPath = 2010 => "invalid-import-path",
        InvalidDecoratorTarget = 2013 => "invalid-decorator-target",
        ConstWithoutInitializer = 2020 => "const-without-initializer",
        InvalidRegex = 2021 => "invalid-regex",
        ForbiddenIndexSignature = 2022 => "forbidden-index-signature",
        TypeMismatch = 3001 => "type-mismatch",
        UnknownSymbol = 3002 => "unknown-symbol",
        NotCallable = 3003 => "not-callable",
        MissingProperty = 3004 => "missing-property",
        NotAssignable = 3005 => "not-assignable",
        NotIndexable = 3006 => "not-indexable",
        CircularTypeReference = 3009 => "circular-type-reference",
        InvalidTypeOperator = 3010 => "invalid-type-operator",
        TypeAnnotationRequired = 3011 => "type-annotation-required",
        InvalidAsCast = 3014 => "invalid-as-cast",
        InvalidSatisfies = 3015 => "invalid-satisfies",
        TupleIndexOutOfBounds = 3016 => "tuple-index-out-of-bounds",
        InfiniteTypeExpansion = 3020 => "infinite-type-expansion",
        IntegerOverflow = 3021 => "integer-overflow",
        VoidValueUsed = 3022 => "void-value-used",
        ForbiddenRecordGeneric = 3023 => "forbidden-record-generic",
        DeprecatedUse = 3025 => "deprecated-use",
        ImpureOperation = 3026 => "impure-operation",
        ConditionNeverFalse = 3027 => "condition-never-false",
        NeverNull = 3028 => "never-null",
        NotIterable = 3029 => "not-iterable",
        UntypedEmptyArray = 3030 => "untyped-empty-array",
        UnusedTask = 3031 => "unused-task",
        InvalidWithEntry = 3032 => "invalid-with-entry",
        DynamicNeedsCast = 3033 => "dynamic-needs-cast",
        AwaitHasNoEffect = 3034 => "await-has-no-effect",
        DuplicateRecordField = 3035 => "duplicate-record-field",
        IsNeedsRuntimeType = 3036 => "is-needs-runtime-type",
        NotHashable = 3037 => "not-hashable",
        InvalidWithReceiver = 3038 => "invalid-with-receiver",
        AsyncReturnNotTask = 3039 => "async-return-not-task",
        RecordRequiresHash = 3040 => "record-requires-hash",
        TypeArgInferenceFailed = 3201 => "type-arg-inference-failed",
        ConstraintViolation = 3202 => "constraint-violation",
        WrongTypeArgCount = 3203 => "wrong-type-arg-count",
        PossibleNullDereference = 3301 => "possible-null-dereference",
        NonExhaustiveMatch = 4001 => "non-exhaustive-match",
        DuplicateDeclaration = 4002 => "duplicate-declaration",
        UseBeforeDeclaration = 4003 => "use-before-declaration",
        UnreachableCode = 4004 => "unreachable-code",
        InvalidBreakTarget = 4005 => "invalid-break-target",
        InvalidContinueTarget = 4006 => "invalid-continue-target",
        ReturnOutsideFunction = 4007 => "return-outside-function",
        InvalidThrowOperand = 4008 => "invalid-throw-operand",
        AwaitOutsideAsync = 4011 => "await-outside-async",
        MatchPatternTypeMismatch = 4015 => "match-pattern-type-mismatch",
        InvalidUsingTarget = 4016 => "invalid-using-target",
        UseBeforeAssignment = 4017 => "use-before-assignment",
        MissingReturn = 4018 => "missing-return",
        NamespaceAsValue = 4020 => "namespace-as-value",
        MutableExport = 4021 => "mutable-export",
        UninitializedField = 4022 => "uninitialized-field",
        InvalidFinallyExit = 4023 => "invalid-finally-exit",
        MissingOverride = 4101 => "missing-override",
        SpuriousOverride = 4102 => "spurious-override",
        PrivateMemberAccess = 4104 => "private-member-access",
        ProtectedMemberAccess = 4105 => "protected-member-access",
        StaticMemberOnInstance = 4106 => "static-member-on-instance",
        AbstractMethodNotImplemented = 4107 => "abstract-method-not-implemented",
        ConstructorNotCalled = 4108 => "constructor-not-called",
        DuplicateMemberName = 4110 => "duplicate-member-name",
        SuperOutsideMethod = 4114 => "super-outside-method",
        ThisOutsideInstance = 4115 => "this-outside-instance",
        IncompatibleOverride = 4116 => "incompatible-override",
        InterfaceNotSatisfied = 4117 => "interface-not-satisfied",
        VarianceViolation = 4118 => "variance-violation",
        CapabilityViolation = 4302 => "capability-violation",
        InvalidDecoratorSignature = 4401 => "invalid-decorator-signature",
        CompilationInternalError = 5005 => "compilation-internal-error",
    }
}

impl ErrorCode {
    pub fn category(self) -> &'static str {
        match self as u32 {
            1000..=1999 => "Lexer",
            2000..=2999 => "Parser",
            3000..=3999 => "TypeChecker",
            4000..=4999 => "Semantic",
            5000..=5999 => "Compiler",
            _ => "General",
        }
    }

    pub fn code_number(self) -> u32 {
        self as u32
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VN{}", *self as u32)
    }
}
