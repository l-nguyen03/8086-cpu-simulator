mod encoding;
mod forms;

use std::fmt;
use std::io;

use forms::{
    alu_op, decode_acc_mem, decode_imm_acc, decode_imm_reg, decode_imm_rm, decode_jump,
    decode_rm_reg, decode_sr_rm,
};

use crate::instruction::{Instruction, Operation};

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    UnknownOpcode(u8),
    UnsupportedAlu(u8),
    InvalidMod(u8),
    InvalidMovRm,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnexpectedEof => write!(f, "truncated instruction"),
            DecodeError::UnknownOpcode(op) => write!(f, "unknown opcode {op:#04x}"),
            DecodeError::UnsupportedAlu(op) => {
                write!(
                    f,
                    "unknown op code for arithmetic immediate to/from acc ({op})"
                )
            }
            DecodeError::InvalidMod(mode) => write!(f, "invalid mod field {mode:#04x}"),
            DecodeError::InvalidMovRm => write!(f, "expected MOV immediate-to-r/m (/0)"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<DecodeError> for io::Error {
    fn from(err: DecodeError) -> Self {
        let kind = match err {
            DecodeError::UnexpectedEof => io::ErrorKind::UnexpectedEof,
            _ => io::ErrorKind::InvalidData,
        };
        io::Error::new(kind, err)
    }
}

pub struct Cursor {
    bytes: Vec<u8>,
    byte_cursor: usize,
    total_byte_length: usize,
}

impl Cursor {
    pub fn new(bytes: Vec<u8>) -> Self {
        let total_byte_length = bytes.len();
        Self {
            bytes,
            byte_cursor: 0,
            total_byte_length,
        }
    }

    pub fn is_finished(&self) -> bool {
        // This means that we have read everything and nothing to execute anymore
        self.byte_cursor >= self.total_byte_length
    }

    pub fn current_position(&self) -> usize {
        self.byte_cursor
    }

    pub fn move_cursor_by(&mut self, offset: i16) {
        self.byte_cursor = self.byte_cursor.saturating_add_signed(offset as isize);
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        if self.byte_cursor < self.total_byte_length {
            let byte = self.bytes[self.byte_cursor];
            self.byte_cursor += 1;
            return Ok(byte);
        }
        Err(DecodeError::UnexpectedEof)
    }

    fn i8(&mut self) -> Result<i8, DecodeError> {
        Ok(self.u8()? as i8)
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        let lo = self.u8()?;
        let hi = self.u8()?;
        Ok(u16::from_le_bytes([lo, hi]))
    }

    fn i16(&mut self) -> Result<i16, DecodeError> {
        Ok(self.u16()? as i16)
    }
}

pub fn instruction(cur: &mut Cursor) -> Result<Instruction, DecodeError> {
    let opcode = cur.u8()?;
    let w = opcode & 1 != 0;
    let d = opcode & 2 != 0;

    match opcode {
        op if op & 0b1111_1100 == 0b1000_1000 => decode_rm_reg(cur, Operation::Mov, d, w),
        op if op & 0b1111_1100 == 0b1010_0000 => decode_acc_mem(cur, d, w),
        op if op & 0b1111_0000 == 0b1011_0000 => decode_imm_reg(cur, (op >> 3) & 1 != 0, op & 7),
        op if op & 0b1111_1110 == 0b1100_0110 => decode_imm_rm(cur, w, false, |reg| {
            if reg == 0 {
                Ok(Operation::Mov)
            } else {
                Err(DecodeError::InvalidMovRm)
            }
        }),
        op if op & 0b1111_1100 == 0b1000_1100 => decode_sr_rm(cur, (op & 0b10) >> 1 == 0b1),
        op if op & 0b1100_0100 == 0b0000_0100 => decode_imm_acc(cur, alu_op(op >> 3)?, w),
        op if op & 0b1111_1100 == 0b1000_0000 => decode_imm_rm(cur, w, d, alu_op),
        op if op & 0b1100_0100 == 0b0000_0000 => decode_rm_reg(cur, alu_op(op >> 3)?, d, w),
        op if op & 0b1111_0000 == 0b0111_0000 => decode_jump(cur, op, false),
        op if op & 0b1111_1100 == 0b1110_0000 => decode_jump(cur, op, true),
        _ => Err(DecodeError::UnknownOpcode(opcode)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_ax_bx_at_eof() {
        let bytes = [0x03, 0xC3];
        let mut cur = Cursor::new(bytes.to_vec());
        let inst = instruction(&mut cur).unwrap();
        assert!(cur.is_finished());
        assert_eq!(inst.to_string(), "add ax, bx");
    }

    #[test]
    fn truncated_rm_reg_is_eof() {
        let mut cur = Cursor::new(Vec::from([0x03]));
        assert!(matches!(
            instruction(&mut cur),
            Err(DecodeError::UnexpectedEof)
        ));
    }
}
