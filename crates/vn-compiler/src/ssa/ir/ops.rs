use super::ids::{BlockId, SsaVar, Value};
use bigdecimal::BigDecimal as Decimal;
use std::sync::Arc;
use vn_tir::{BackendTy, DynReason};

#[derive(Debug, Clone)]
pub enum InstKind {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstStr(Arc<str>),
    ConstChar(char),
    ConstDecimal(Decimal),

    ConstBigInt(Arc<str>),
    ConstNull,
    Binary {
        op: vn_core::BinOp,
        lhs: Value,
        rhs: Value,
        ty: BackendTy,
    },
    Unary {
        op: vn_core::UnOp,
        operand: Value,
        ty: BackendTy,
    },

    LoadGlobal(Arc<str>),

    LoadGlobalIdx(u32),

    LoadNativeGlobalIdx(u32),

    LoadUpvalue(u32),

    StoreGlobal {
        name: Arc<str>,
        value: Value,
    },

    StoreGlobalIdx {
        slot: u32,
        value: Value,
    },

    StoreUpvalue {
        index: u32,
        value: Value,
    },

    Call {
        callee: Value,
        args: Vec<Value>,
        shape: Option<u32>,
    },

    CallDirect {
        target: Arc<str>,
        slot: Option<u32>,
        callee: Value,
        args: Vec<Value>,
        shape: Option<u32>,
    },

    AllocInstance {
        class: Value,
        payload_size: u32,
    },

    SelfCall {
        args: Vec<Value>,
    },

    GetProperty {
        object: Value,
        name: Arc<str>,
    },

    GetSlotMethod {
        object: Value,
        slot: u16,
    },

    GetFixedField {
        object: Value,
        slot: u16,

        offset: u32,
        tag: vn_core::FieldAccess,
    },

    GetIndex {
        object: Value,
        index: Value,
    },

    ArrayGetIndex {
        object: Value,
        index: Value,
    },

    BytesGetIndex {
        object: Value,
        index: Value,
    },

    MapGetIndex {
        object: Value,
        index: Value,
    },

    SetProperty {
        object: Value,
        name: Arc<str>,
        value: Value,
    },

    SetFixedField {
        object: Value,
        value: Value,
        slot: u16,

        offset: u32,

        tag: Option<vn_core::FieldKind>,
    },

    SetIndex {
        object: Value,
        index: Value,
        value: Value,
    },

    ArraySetIndex {
        object: Value,
        index: Value,
        value: Value,
    },

    MapSetIndex {
        object: Value,
        index: Value,
        value: Value,
    },

    BytesSetIndex {
        object: Value,
        index: Value,
        value: Value,
    },

    ArrayPush {
        array: Value,
        value: Value,
    },

    ObjectMerge {
        target: Value,
        source: Value,
    },

    MethodCall {
        recv: Value,
        name: Arc<str>,
        args: Vec<Value>,
    },

    InvokeSlot {
        recv: Value,
        slot: u16,
        args: Vec<Value>,
    },

    IsNull {
        operand: Value,
    },

    TypeTest {
        value: Value,
        class: Value,
    },

    Cast {
        operand: Value,
        ty: BackendTy,
    },

    Convert {
        operand: Value,
        conv: vn_core::ValueConv,
    },

    BuildArray {
        elements: Vec<Value>,
        storage: vn_core::ArrayStorage,
    },

    BuildTuple {
        elements: Vec<Value>,
    },

    BuildObject {
        pairs: Vec<(Arc<str>, Value)>,
    },

    BuildMap {
        pairs: Vec<(Value, Value)>,
    },

    ObjectRest {
        object: Value,
        skip_keys: Vec<Arc<str>>,
    },

    ToString {
        operand: Value,
    },

    BuildStr {
        parts: Vec<Value>,
    },

    MakeClosure {
        func: u32,
        upvalues_src: Vec<vn_tir::TirUpvalue>,
    },
    LoadCaptured {
        var: SsaVar,
    },
    StoreCaptured {
        var: SsaVar,
        value: Value,
    },
    MakeClass {
        name: Arc<str>,
        super_class: Option<Value>,
    },
    DeclareLayout {
        class: Value,
        layout: Arc<vn_core::layout::ClassLayout>,
    },
    FreezeClass {
        class: Value,
    },
    DefineMember {
        class: Value,
        name: Arc<str>,
        value: Value,
        kind: vn_core::MemberDef,
    },
    MakeEnumVariant {
        def: Arc<vn_types::value::VariantDef>,
    },
    Try {
        handler: BlockId,
    },
    PopTry,
    CatchParam {
        try_val: Value,
    },
    CloseUpvalues {
        targets: Vec<SsaVar>,
    },
    LoadModule {
        source: Arc<str>,
    },
    StoreModuleSlot {
        value: Value,
        slot: u16,
    },
    Await {
        operand: Value,
    },
    Spawn {
        operand: Value,
    },
    Yield {
        operand: Value,
    },

    IntrinsicCall {
        object: Value,
        args: Vec<Value>,
        wire_byte: u8,
    },

    CallNativeOp {
        object: Value,
        args: Vec<Value>,
        op_id: u64,
    },

    AssertNotNull {
        operand: Value,
    },

    GetPropertyMaybe {
        object: Value,
        name: Arc<str>,
    },

    ModuleSlot {
        object: Value,
        slot: u16,
    },

    GetEnumTag {
        operand: Value,
    },

    IsArray {
        operand: Value,
    },

    StrLength {
        operand: Value,
    },

    ArrayLength {
        operand: Value,
    },

    BytesLength {
        operand: Value,
    },

    This,

    Range {
        start: Value,
        end: Value,
        inclusive: bool,
    },

    ObjectKeys {
        operand: Value,
    },

    GetSuper {
        name: Arc<str>,
    },

    GetSuperSlot {
        slot: u16,
    },

    SuperCall {
        args: Vec<Value>,
    },

    SuperMethodCall {
        name: Arc<str>,
        args: Vec<Value>,
    },

    SuperInvokeSlot {
        slot: u16,
        args: Vec<Value>,
    },

    ExtensionCall {
        func: Arc<str>,

        slot: Option<u32>,
        recv: Value,
        args: Vec<Value>,
    },

    CallSpread {
        callee: Value,
        args: Vec<vn_types::ssa::SsaSpread<Value>>,
    },

    BuildArraySpread {
        elements: Vec<vn_types::ssa::SsaSpread<Value>>,
    },

    BuildObjectSpread {
        parts: Vec<vn_types::ssa::SsaObjectSpreadPart<Value>>,
    },
}

impl InstKind {
    pub(crate) fn const_ty(&self) -> Option<BackendTy> {
        match self {
            InstKind::ConstInt(_) => Some(BackendTy::INT),
            InstKind::ConstFloat(_) => Some(BackendTy::FLOAT),
            InstKind::ConstBool(_) => Some(BackendTy::Bool),
            InstKind::ConstStr(_) => Some(BackendTy::Str),

            InstKind::ConstChar(_) => Some(BackendTy::Char),
            InstKind::ConstDecimal(_) | InstKind::ConstBigInt(_) => {
                Some(BackendTy::Dynamic(DynReason::Boxed))
            }
            InstKind::ConstNull => Some(BackendTy::Dynamic(DynReason::Boxed)),
            InstKind::Binary { .. }
            | InstKind::Unary { .. }
            | InstKind::LoadGlobal(_)
            | InstKind::LoadGlobalIdx(_)
            | InstKind::LoadNativeGlobalIdx(_)
            | InstKind::LoadUpvalue(_)
            | InstKind::StoreGlobal { .. }
            | InstKind::StoreGlobalIdx { .. }
            | InstKind::StoreUpvalue { .. }
            | InstKind::Call { .. }
            | InstKind::CallDirect { .. }
            | InstKind::AllocInstance { .. }
            | InstKind::SelfCall { .. }
            | InstKind::GetProperty { .. }
            | InstKind::GetSlotMethod { .. }
            | InstKind::GetFixedField { .. }
            | InstKind::GetIndex { .. }
            | InstKind::ArrayGetIndex { .. }
            | InstKind::BytesGetIndex { .. }
            | InstKind::MapGetIndex { .. }
            | InstKind::SetProperty { .. }
            | InstKind::SetFixedField { .. }
            | InstKind::SetIndex { .. }
            | InstKind::ArraySetIndex { .. }
            | InstKind::BytesSetIndex { .. }
            | InstKind::MapSetIndex { .. }
            | InstKind::ArrayPush { .. }
            | InstKind::ObjectMerge { .. }
            | InstKind::MethodCall { .. }
            | InstKind::InvokeSlot { .. }
            | InstKind::IsNull { .. }
            | InstKind::TypeTest { .. }
            | InstKind::Cast { .. }
            | InstKind::Convert { .. }
            | InstKind::BuildArray { .. }
            | InstKind::BuildTuple { .. }
            | InstKind::BuildObject { .. }
            | InstKind::BuildMap { .. }
            | InstKind::ObjectRest { .. }
            | InstKind::ToString { .. }
            | InstKind::BuildStr { .. }
            | InstKind::MakeClosure { .. }
            | InstKind::LoadCaptured { .. }
            | InstKind::StoreCaptured { .. }
            | InstKind::MakeClass { .. }
            | InstKind::DeclareLayout { .. }
            | InstKind::FreezeClass { .. }
            | InstKind::DefineMember { .. }
            | InstKind::MakeEnumVariant { .. }
            | InstKind::Try { .. }
            | InstKind::PopTry
            | InstKind::CatchParam { .. }
            | InstKind::CloseUpvalues { .. }
            | InstKind::LoadModule { .. }
            | InstKind::StoreModuleSlot { .. }
            | InstKind::Await { .. }
            | InstKind::Spawn { .. }
            | InstKind::Yield { .. }
            | InstKind::IntrinsicCall { .. }
            | InstKind::CallNativeOp { .. }
            | InstKind::AssertNotNull { .. }
            | InstKind::GetPropertyMaybe { .. }
            | InstKind::ModuleSlot { .. }
            | InstKind::GetEnumTag { .. }
            | InstKind::IsArray { .. }
            | InstKind::StrLength { .. }
            | InstKind::ArrayLength { .. }
            | InstKind::BytesLength { .. }
            | InstKind::This
            | InstKind::Range { .. }
            | InstKind::ObjectKeys { .. }
            | InstKind::GetSuper { .. }
            | InstKind::GetSuperSlot { .. }
            | InstKind::SuperCall { .. }
            | InstKind::SuperMethodCall { .. }
            | InstKind::SuperInvokeSlot { .. }
            | InstKind::ExtensionCall { .. }
            | InstKind::CallSpread { .. }
            | InstKind::BuildArraySpread { .. }
            | InstKind::BuildObjectSpread { .. } => None,
        }
    }
}
