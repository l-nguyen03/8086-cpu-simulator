use std::io;

use super::encoding::{
    ACC_REG, REGISTER_MAPS, decode_imm, decode_rm_field, decompose_instruction_field, format_error,
    with_size_prefix,
};
use crate::instruction::Instruction;

fn decode_arithmetic_opcode(byte: u8) -> io::Result<String> {
    const ARITHMETIC_OP_MASK: u8 = 0b0011_1000;
    let op_code = byte & ARITHMETIC_OP_MASK;
    match op_code {
        0b0000_0000 => Ok("add".to_string()),
        0b0010_1000 => Ok("sub".to_string()),
        0b0011_1000 => Ok("cmp".to_string()),
        _ => Err(format_error(
            io::ErrorKind::InvalidData,
            "unknown op code for arithmetic immediate to/from acc",
        )),
    }
}

pub(super) fn decode_arithmetic_immediate_and_acc(
    bytes: &[u8],
) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for immediate and accumulator arithmetic",
        ));
    }

    let op_name = decode_arithmetic_opcode(bytes[0])?;

    let w = (bytes[0] & 1) != 0;
    let acc = ACC_REG[usize::from(w)];

    let (imm, imm_len) = decode_imm(bytes, w, false, 1)?;

    Ok((Instruction::binary(op_name, acc, imm), imm_len + 1))
}

pub(super) fn decode_arithmetic_immediate_and_rm(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 3 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for immediate and register/memory arithmetic",
        ));
    }

    let fields = decompose_instruction_field(bytes)?;
    let op_name = decode_arithmetic_opcode(bytes[1])?;

    let (dst, disp_len) = decode_rm_field(bytes, &fields)?;
    let dst = with_size_prefix(dst, fields.mode != 0b11, fields.w_bit);
    let (imm, imm_len) = decode_imm(bytes, fields.w_bit, fields.d_bit, disp_len + 2)?;

    Ok((
        Instruction::binary(op_name, dst, imm),
        imm_len + disp_len + 2,
    ))
}

pub(super) fn decode_arithmetic_rm_and_reg(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 3 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for immediate and register/memory arithmetic",
        ));
    }

    let fields = decompose_instruction_field(bytes)?;
    let op_name = decode_arithmetic_opcode(bytes[0])?;

    let register_map = &REGISTER_MAPS[usize::from(fields.w_bit)];
    let reg = register_map[fields.reg_field].to_string();
    let (rm, disp_len) = decode_rm_field(bytes, &fields)?;

    let (dst, src) = if fields.d_bit { (reg, rm) } else { (rm, reg) };
    Ok((Instruction::binary(op_name, dst, src), disp_len + 2))
}
