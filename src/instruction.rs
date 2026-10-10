use std::fmt;

pub(super) const BYTE_REGISTER: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
pub(super) const WORD_REGISTER: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];
pub(super) const SEGMENT_REGISTER: [&str; 4] = ["es", "cs", "ss", "ds"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Byte,
    Word,
}

#[derive(Clone, Copy)]
pub enum Operand {
    SegmentRegister {
        index: usize,
    },
    GeneralRegister {
        index: usize,
        width: Width,
    },
    Memory {
        ea: Option<&'static str>,
        disp: i16,
        direct: bool,
        size: Option<Width>,
    },
    Imm(i16),
}

#[derive(Clone, Copy)]
pub enum Operation {
    Mov,
    Add,
    Sub,
    Cmp,
    Jo,
    Jno,
    Jb,
    Jnb,
    Je,
    Jne,
    Jbe,
    Jnbe,
    Js,
    Jns,
    Jp,
    Jnp,
    Jl,
    Jnl,
    Jle,
    Jnle,
    Loopnz,
    Loopz,
    Loop,
    Jcxz,
}

#[derive(Clone, Copy)]
pub struct Instruction {
    pub mnemonic: Operation,
    pub dst: Operand,
    pub src: Option<Operand>,
}

impl Instruction {
    pub fn binary(mnemonic: Operation, dst: Operand, src: Operand) -> Self {
        Self {
            mnemonic,
            dst,
            src: Some(src),
        }
    }

    pub fn unary(mnemonic: Operation, dst: Operand) -> Self {
        Self {
            mnemonic,
            dst,
            src: None,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.src {
            Some(src) => write!(f, "{} {}, {}", self.mnemonic, self.dst, src),
            None => write!(f, "{} {}", self.mnemonic, self.dst),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::GeneralRegister { index, width } => match width {
                Width::Byte => write!(f, "{}", BYTE_REGISTER[*index]),
                Width::Word => write!(f, "{}", WORD_REGISTER[*index]),
            },
            Operand::SegmentRegister { index } => write!(f, "{}", SEGMENT_REGISTER[*index]),
            Operand::Imm(value) => write!(f, "{value}"),
            Operand::Memory {
                ea,
                disp,
                direct,
                size,
            } => {
                match size {
                    Some(Width::Byte) => write!(f, "byte ")?,
                    Some(Width::Word) => write!(f, "word ")?,
                    None => {}
                }
                if *direct {
                    write!(f, "[{}]", *disp as u16)
                } else {
                    let ea = ea.expect("effective-address memory has a base");
                    match disp {
                        0 => write!(f, "[{ea}]"),
                        d if *d < 0 => write!(f, "[{ea} - {}]", d.unsigned_abs()),
                        d => write!(f, "[{ea} + {d}]"),
                    }
                }
            }
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Mov => write!(f, "mov"),
            Operation::Add => write!(f, "add"),
            Operation::Sub => write!(f, "sub"),
            Operation::Cmp => write!(f, "cmp"),
            Operation::Jo => write!(f, "jo"),
            Operation::Jno => write!(f, "jno"),
            Operation::Jb => write!(f, "jb"),
            Operation::Jnb => write!(f, "jnb"),
            Operation::Je => write!(f, "je"),
            Operation::Jne => write!(f, "jne"),
            Operation::Jbe => write!(f, "jbe"),
            Operation::Jnbe => write!(f, "jnbe"),
            Operation::Js => write!(f, "js"),
            Operation::Jns => write!(f, "jns"),
            Operation::Jp => write!(f, "jp"),
            Operation::Jnp => write!(f, "jnp"),
            Operation::Jl => write!(f, "jl"),
            Operation::Jnl => write!(f, "jnl"),
            Operation::Jle => write!(f, "jle"),
            Operation::Jnle => write!(f, "jnle"),
            Operation::Loopnz => write!(f, "loopnz"),
            Operation::Loopz => write!(f, "loopz"),
            Operation::Loop => write!(f, "loop"),
            Operation::Jcxz => write!(f, "jcxz"),
        }
    }
}
