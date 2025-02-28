


// Binary Gcode

pub struct Instruction{
    pub opcode: Opcode,
    pub oprand: Operand
}

pub union Opcode{
    builtin: BuiltinOpcode,
    external: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinOpcode{
    prefix: u8,
    index: u32,
    suffix: u8,
}

impl BuiltinOpcode{
    pub const G1: Self = Self{ prefix: b'G', index: 1, suffix: 0};
    pub const G4: Self = Self{ prefix: b'G', index: 4, suffix: 0};
    pub const G28: Self = Self{ prefix: b'G', index: 28, suffix: 0};
    pub const G90: Self = Self{ prefix: b'G', index: 90, suffix: 0};
    pub const G91: Self = Self{ prefix: b'G', index: 91, suffix: 0};
    pub const G92: Self = Self{ prefix: b'G', index: 92, suffix: 0};
}

#[derive(Debug)]
pub struct Operand{
    /// flags
    pub flags: u16,
    /// length of operand in bytes
    pub length: u16,
    /// pointer to the operand data
    pub pointer: u32,
}

#[derive(Debug)]
pub struct G1Params{
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub e: f32,
    pub f: f32
}

#[derive(Debug)]
pub struct G4Params{
    pub p: f32,
    pub s: f32,
}

#[derive(Debug)]
pub struct G28Params{
    pub x: bool,
    pub y: bool,
    pub z: bool,
}

#[derive(Debug)]
pub struct G92Params{
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub e: f32,
}