use serde::{Deserialize, Serialize};

use super::operands::{SsaObjectSpreadPart, SsaSpread, SsaUpvalue, SsaVar};
use super::operators::{SsaBinOp, SsaUnOp};
use super::ValueId;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SsaOp {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstNull,
    ConstStr(std::sync::Arc<str>),

    Binary {
        op: SsaBinOp,
        lhs: ValueId,
        rhs: ValueId,
    },

    Unary {
        op: SsaUnOp,
        operand: ValueId,
    },

    SelfCall {
        args: Vec<ValueId>,
    },

    Call {
        callee: ValueId,
        args: Vec<ValueId>,
    },

    CallDirect {
        target: std::sync::Arc<str>,
        slot: Option<u32>,
        callee: ValueId,
        args: Vec<ValueId>,
    },

    AllocInstance {
        class: ValueId,
        payload_size: u32,
    },

    LoadGlobalIdx(u32),

    ArrayGetIndex {
        object: ValueId,
        index: ValueId,
    },

    BytesGetIndex {
        object: ValueId,
        index: ValueId,
    },

    ArraySetIndex {
        object: ValueId,
        index: ValueId,
        value: ValueId,
    },

    BytesSetIndex {
        object: ValueId,
        index: ValueId,
        value: ValueId,
    },

    ConstChar(char),

    ConstBigInt(std::sync::Arc<str>),

    ConstDecimal(bigdecimal::BigDecimal),

    MakeEnumVariant {
        def: std::sync::Arc<crate::value::VariantDef>,
    },

    IntrinsicCall {
        object: ValueId,
        args: Vec<ValueId>,
        wire: u8,
    },

    LoadNativeGlobalIdx(u32),

    Try {
        catch_value: ValueId,
    },

    PopTry,

    CatchParam {
        try_val: ValueId,
    },

    StoreGlobalIdx {
        slot: u32,
        value: ValueId,
    },

    MakeClosure {
        proto: u32,
        upvalues: Vec<SsaUpvalue>,
    },

    LoadCaptured {
        var: SsaVar,
    },

    StoreCaptured {
        var: SsaVar,
        value: ValueId,
    },

    LoadUpvalue(u32),

    StoreUpvalue {
        index: u32,
        value: ValueId,
    },

    CloseUpvalues {
        vars: Vec<SsaVar>,
    },

    Cast {
        operand: ValueId,
    },

    Convert {
        operand: ValueId,
        conv: vn_core::ValueConv,
    },

    IsNull {
        operand: ValueId,
    },

    TypeTest {
        value: ValueId,
        class: ValueId,
    },

    KindOf {
        operand: ValueId,
    },

    ToString {
        operand: ValueId,
    },

    IsArray {
        operand: ValueId,
    },

    GetEnumTag {
        operand: ValueId,
    },

    ObjectKeys {
        operand: ValueId,
    },

    BuildStr {
        parts: Vec<ValueId>,
    },

    BuildArray {
        elements: Vec<ValueId>,
        storage: vn_core::ArrayStorage,
    },

    BuildMap {
        pairs: Vec<(ValueId, ValueId)>,
    },

    BuildObject {
        pairs: Vec<(std::sync::Arc<str>, ValueId)>,
    },

    GetProperty {
        object: ValueId,
        name: std::sync::Arc<str>,
    },

    GetSlotMethod {
        object: ValueId,
        slot: u16,
    },

    SetProperty {
        object: ValueId,
        value: ValueId,
        name: std::sync::Arc<str>,
    },

    GetIndex {
        object: ValueId,
        index: ValueId,
    },

    SetIndex {
        object: ValueId,
        index: ValueId,
        value: ValueId,
    },

    ArrayLength {
        operand: ValueId,
    },

    StrLength {
        operand: ValueId,
    },

    BytesLength {
        operand: ValueId,
    },

    MethodCall {
        recv: ValueId,
        name: std::sync::Arc<str>,
        args: Vec<ValueId>,
    },

    InvokeSlot {
        recv: ValueId,
        slot: u16,
        args: Vec<ValueId>,
    },

    CallNativeOp {
        object: ValueId,
        args: Vec<ValueId>,
        op_id: u64,
    },

    ArrayPush {
        array: ValueId,
        value: ValueId,
    },

    This,

    GetFixedField {
        object: ValueId,
        slot: u16,
        offset: u32,
        access: vn_core::FieldAccess,
    },

    SetFixedField {
        object: ValueId,
        value: ValueId,
        slot: u16,
        offset: u32,
        kind: Option<vn_core::FieldKind>,
    },

    MakeClass {
        name: std::sync::Arc<str>,
        super_class: Option<ValueId>,
    },

    FreezeClass {
        class: ValueId,
    },

    DeclareLayout {
        class: ValueId,
        layout: std::sync::Arc<vn_core::layout::ClassLayout>,
    },

    DefineMember {
        class: ValueId,
        name: std::sync::Arc<str>,
        member: ValueId,
        kind: vn_core::MemberDef,
    },

    GetSuper {
        name: std::sync::Arc<str>,
    },

    GetSuperSlot {
        slot: u16,
    },

    LoadGlobal(std::sync::Arc<str>),

    StoreGlobal {
        name: std::sync::Arc<str>,
        value: ValueId,
    },

    BuildTuple {
        elements: Vec<ValueId>,
    },

    BuildArraySpread {
        elements: Vec<SsaSpread>,
    },

    BuildObjectSpread {
        parts: Vec<SsaObjectSpreadPart>,
    },

    ObjectMerge {
        target: ValueId,
        source: ValueId,
    },

    ObjectRest {
        object: ValueId,
        skip_keys: Vec<std::sync::Arc<str>>,
    },

    GetPropertyMaybe {
        object: ValueId,
        name: std::sync::Arc<str>,
    },

    AssertNotNull {
        operand: ValueId,
    },

    BindMethod {
        object: ValueId,
        name: std::sync::Arc<str>,
    },

    ArrayExtend {
        array: ValueId,
        source: ValueId,
    },

    WrapSpread {
        operand: ValueId,
    },

    Range {
        start: ValueId,
        end: ValueId,
        inclusive: bool,
    },

    SuperCall {
        args: Vec<ValueId>,
    },

    SuperMethodCall {
        name: std::sync::Arc<str>,
        args: Vec<ValueId>,
    },

    SuperInvokeSlot {
        slot: u16,
        args: Vec<ValueId>,
    },

    ExtensionCall {
        func: std::sync::Arc<str>,
        slot: Option<u32>,
        recv: ValueId,
        args: Vec<ValueId>,
    },

    CallSpread {
        callee: ValueId,
        args: Vec<SsaSpread>,
    },

    LoadModule {
        source: std::sync::Arc<str>,
    },

    ModuleSlot {
        object: ValueId,
        slot: u16,
    },

    StoreModuleSlot {
        slot: u16,
        value: ValueId,
    },

    Await {
        operand: ValueId,
    },

    Spawn {
        operand: ValueId,
    },

    Yield {
        operand: ValueId,
    },
}
