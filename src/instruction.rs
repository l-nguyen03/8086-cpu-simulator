use std::fmt;

#[derive(Clone, Copy)]
pub enum Width {
    Byte,
    Word,
}

pub enum Operand {
    Register(&'static str),
    Memory {
        ea: Option<&'static str>,
        disp: i16,
        direct: bool,
        size: Option<Width>,
    },
    Imm(i16),
}

pub struct Instruction {
    mnemonic: &'static str,
    dst: Operand,
    src: Option<Operand>,
}

impl Instruction {
    pub fn binary(mnemonic: &'static str, dst: Operand, src: Operand) -> Self {
        Self {
            mnemonic,
            dst,
            src: Some(src),
        }
    }

    pub fn unary(mnemonic: &'static str, dst: Operand) -> Self {
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
            Operand::Register(name) => write!(f, "{name}"),
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
