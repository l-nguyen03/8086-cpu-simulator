mod arithmetic;
mod encoding;
mod jump;
mod mov;

use std::io;

use arithmetic::{
    decode_arithmetic_immediate_and_acc, decode_arithmetic_immediate_and_rm,
    decode_arithmetic_rm_and_reg,
};
use encoding::format_error;
use jump::{decode_cond_jmp, decode_loop};
use mov::{
    decode_acc_mem_mov, decode_immediate_reg_mov, decode_immediate_rm_mov, decode_reg_to_reg_mov,
};

use crate::instruction::Instruction;

const OP_CODE_MASK: u8 = 0b1111_1100;

pub fn instruction(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    let opcode = bytes[0];
    match opcode & OP_CODE_MASK {
        0b1000_1000 => decode_reg_to_reg_mov(bytes),
        0b1010_0000 => decode_acc_mem_mov(bytes),
        _ if (opcode & 0b1111_1110) == 0b1100_0110 => decode_immediate_rm_mov(bytes),
        _ if (opcode & 0b1111_0000) == 0b1011_0000 => decode_immediate_reg_mov(bytes),
        _ if (opcode & 0b1100_0100) == 0b0000_0100 => decode_arithmetic_immediate_and_acc(bytes),
        0b1000_0000 => decode_arithmetic_immediate_and_rm(bytes),
        _ if (opcode & 0b1100_0100) == 0b0000_0000 => decode_arithmetic_rm_and_reg(bytes),
        _ if (opcode & 0b1111_0000 == 0b0111_0000) => decode_cond_jmp(bytes),
        0b1110_0000 => decode_loop(bytes),
        _ => Err(format_error(
            io::ErrorKind::InvalidData,
            &format!("unknown opcode {opcode:#04x}"),
        )),
    }
}
