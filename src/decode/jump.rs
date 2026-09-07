use std::io;

use super::encoding::format_error;
use crate::instruction::Instruction;

const COND_JUMP_LOOKUP: [&str; 16] = [
    "jo", "jno", "jb", "jnb", "je", "jne", "jbe", "jnbe", "js", "jns", "jp", "jnp", "jl", "jnl",
    "jle", "jnle",
];

const LOOP_JMP_LOOKUP: [&str; 4] = ["loopnz", "loopz", "loop", "jcxz"];

pub(super) fn decode_cond_jmp(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for conditional jump",
        ));
    }

    let op = COND_JUMP_LOOKUP[(bytes[0] & 0b0000_1111) as usize];
    Ok((Instruction::unary(op, (bytes[1] as i8).to_string()), 2))
}

pub(super) fn decode_loop(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for loop instructions",
        ));
    }

    let op = LOOP_JMP_LOOKUP[(bytes[0] & 0b0000_1111) as usize];
    Ok((Instruction::unary(op, (bytes[1] as i8).to_string()), 2))
}
