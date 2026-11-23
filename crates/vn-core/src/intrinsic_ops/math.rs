crate::named_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum MathOp {
        Abs = 0x0 => "abs",
        Sqrt = 0x1 => "sqrt",
        Floor = 0x2 => "floor",
        Ceil = 0x3 => "ceil",
        Round = 0x4 => "round",
        Sin = 0x5 => "sin",
        Cos = 0x6 => "cos",
        Tan = 0x7 => "tan",
        Log = 0x8 => "log",
        Exp = 0x9 => "exp",
        Pow = 0xA => "pow",
        Min = 0xB => "min",
        Max = 0xC => "max",
    }
}

impl MathOp {
    pub const MODULE: &'static str = "std:math";

    pub const fn wire(self) -> u8 {
        self as u8
    }

    pub fn of_wire(wire: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.wire() == wire)
    }
}

pub fn is_unary_math(wire_byte: u8) -> bool {
    let op = wire_byte;
    op == MathOp::Abs as u8
        || op == MathOp::Sqrt as u8
        || op == MathOp::Floor as u8
        || op == MathOp::Ceil as u8
}
