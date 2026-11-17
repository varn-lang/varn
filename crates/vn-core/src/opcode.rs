#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpCode {
    LoadConst,
    LoadNull,
    LoadTrue,
    LoadFalse,
    LoadInt,
    Move,
    LoadGlobal,
    StoreGlobal,
    DefineGlobal,
    DefineGlobalIdx,
    LoadGlobalIdx,
    StoreGlobalIdx,
    LoadUpvalue,
    StoreUpvalue,
    CloseUpvalue,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Negate,
    Not,
    ToString,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Ushr,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Loop,
    Call,
    CallMethod,
    InvokeVirtual,
    InvokeSlot,
    SuperInvokeSlot,
    CallSpread,
    Return,
    BuildArray,
    BuildTuple,
    BuildObject,
    BuildObjectWithShape,
    GetIndex,
    SetIndex,
    ObjectRest,
    ObjectKeys,
    ObjectMerge,
    GetProperty,
    GetPropertyMaybe,
    SetProperty,
    GetFixedField,
    SetFixedField,
    GetSuper,

    MakeClosure,

    MakeClass,

    Inherit,

    Method,

    DefineStatic,

    DefineGetter,

    DefineSetter,

    DefineStaticGetter,

    DefineStaticSetter,

    DeclareLayout,

    AllocInstance,

    FreezeClass,

    BindMethod,

    Typeof,

    Instanceof,

    In,

    IsNull,

    IsArray,

    AssertNotNull,

    StrConcat,

    StrLength,

    StrSlice,

    ArrayLength,

    ArrayPush,

    ArrayPop,

    ArrayExtend,

    WrapSpread,

    MakeEnumVariant,

    GetEnumTag,

    Await,

    Spawn,

    Yield,

    Try,

    Throw,

    PopTry,

    LoadModule,

    LoadModuleSlot,

    StoreModuleSlot,

    InvokeRuntimeStatic,

    AddImm,
    SubImm,

    BuildStr,

    LoadIntZero,
    LoadIntOne,
    LoadIntMinusOne,

    AddInt,
    SubInt,
    MulInt,
    DivInt,
    LtInt,
    GtInt,
    LteInt,
    GteInt,
    EqInt,
    NeqInt,

    AddFloat,
    SubFloat,
    MulFloat,
    DivFloat,
    ModFloat,
    PowFloat,
    LtFloat,
    GtFloat,
    LteFloat,
    GteFloat,
    EqFloat,
    NeqFloat,

    EqStr,
    NeqStr,

    ModInt,
    PowInt,

    Intrinsic,

    LoadStaticFn,

    CallSelf,

    Nop,

    ArrayGetIndex,
    ArraySetIndex,

    BytesGetIndex,
    BytesSetIndex,

    CallNativeOp,

    IntrinsicDirect,

    LoadNativeGlobalIdx,

    BuildMap,
    MapGetIndex,
    MapSetIndex,

    Convert,

    BytesLength,

    LtImm,
    LteImm,
    GtImm,
    GteImm,
    EqImm,
    NeqImm,

    EqBool,
    NeqBool,
    EqChar,
    NeqChar,

    GetSlotMethod,
    GetSuperSlot,
}

impl OpCode {
    #[inline(always)]
    pub fn from_u8(v: u8) -> Option<Self> {
        if v <= OpCode::GetSuperSlot as u8 {
            Some(unsafe { std::mem::transmute::<u8, OpCode>(v) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn from_u16(v: u16) -> Option<Self> {
        Self::from_u8(v as u8)
    }

    /// Opcode without validation, for the interpreter hot loop.
    ///
    /// # Safety
    ///
    /// `v` must be a valid opcode discriminant of a chunk that passed
    /// `vn_types::bytecode::verify_chunk`. Every validated instruction decodes
    /// to a known opcode, so the loop only ever reads valid discriminants.
    #[inline(always)]
    pub unsafe fn from_u8_unchecked(v: u8) -> Self {
        debug_assert!(Self::from_u8(v).is_some());
        unsafe { std::mem::transmute::<u8, OpCode>(v) }
    }
}
