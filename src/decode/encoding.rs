use super::{Cursor, DecodeError};
use crate::instruction::{Operand, Width};

const EFFECTIVE_ADDRESS: [&str; 8] = [
    "bx + si", "bx + di", "bp + si", "bp + di", "si", "di", "bp", "bx",
];

pub(super) struct ModRm {
    pub(super) mode: u8,
    pub(super) reg: u8,
    pub(super) rm: u8,
}

impl ModRm {
    pub(super) fn from_byte(byte: u8) -> Self {
        Self {
            mode: byte >> 6,
            reg: (byte >> 3) & 7,
            rm: byte & 7,
        }
    }
}

pub(super) fn register(wide: bool, index: u8) -> Operand {
    Operand::GeneralRegister {
        index: index as usize,
        width: if wide { Width::Word } else { Width::Byte },
    }
}

pub(super) fn acc(wide: bool) -> Operand {
    if wide {
        Operand::GeneralRegister {
            index: 0,
            width: Width::Word,
        }
    } else {
        Operand::GeneralRegister {
            index: 0,
            width: Width::Byte,
        }
    }
}

pub(super) fn read_modrm(cur: &mut Cursor<'_>) -> Result<ModRm, DecodeError> {
    Ok(ModRm::from_byte(cur.u8()?))
}

pub(super) fn read_rm(
    cur: &mut Cursor<'_>,
    modrm: &ModRm,
    wide: bool,
) -> Result<Operand, DecodeError> {
    match modrm.mode {
        0b11 => Ok(register(wide, modrm.rm)),
        0b01 => Ok(memory_ea(modrm.rm, i16::from(cur.i8()?))),
        0b10 => Ok(memory_ea(modrm.rm, cur.i16()?)),
        0b00 => {
            if modrm.rm == 0b110 {
                Ok(memory_direct(cur.u16()?))
            } else {
                Ok(memory_ea(modrm.rm, 0))
            }
        }
        mode => Err(DecodeError::InvalidMod(mode)),
    }
}

pub(super) fn read_imm(
    cur: &mut Cursor<'_>,
    wide: bool,
    sign_extend: bool,
) -> Result<i16, DecodeError> {
    match (wide, sign_extend) {
        (true, false) => Ok(cur.i16()?),
        _ => Ok(i16::from(cur.i8()?)),
    }
}

pub(super) fn with_size(operand: Operand, wide: bool) -> Operand {
    match operand {
        Operand::Memory {
            ea, disp, direct, ..
        } => Operand::Memory {
            ea,
            disp,
            direct,
            size: Some(if wide { Width::Word } else { Width::Byte }),
        },
        other => other,
    }
}

fn memory_ea(rm: u8, disp: i16) -> Operand {
    Operand::Memory {
        ea: Some(EFFECTIVE_ADDRESS[usize::from(rm)]),
        disp,
        direct: false,
        size: None,
    }
}

fn memory_direct(addr: u16) -> Operand {
    Operand::Memory {
        ea: None,
        disp: addr as i16,
        direct: true,
        size: None,
    }
}
