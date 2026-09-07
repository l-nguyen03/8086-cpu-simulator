use std::io;

use super::encoding::{
    ACC_REG, REGISTER_MAPS, decode_imm, decode_rm_field, decompose_instruction_field, format_error,
    with_size_prefix,
};
use crate::instruction::Instruction;

pub(super) fn decode_reg_to_reg_mov(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    let fields = decompose_instruction_field(bytes)?;
    let register_map = &REGISTER_MAPS[usize::from(fields.w_bit)];
    let reg = register_map[fields.reg_field].to_string();
    let (rm, disp_len) = decode_rm_field(bytes, &fields)?;
    let (dst, src) = if fields.d_bit { (reg, rm) } else { (rm, reg) };
    Ok((Instruction::binary("mov", dst, src), 2 + disp_len))
}

pub(super) fn decode_immediate_rm_mov(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    let fields = decompose_instruction_field(bytes)?;
    if fields.reg_field != 0 {
        return Err(format_error(
            io::ErrorKind::InvalidData,
            "expected MOV immediate-to-r/m (/0)",
        ));
    }

    let (dst, disp_len) = decode_rm_field(bytes, &fields)?;
    let dst = with_size_prefix(dst, fields.mode != 0b11, fields.w_bit);
    let (imm, imm_len) = decode_imm(bytes, fields.w_bit, false, disp_len + 2)?;
    Ok((Instruction::binary("mov", dst, imm), imm_len + disp_len + 2))
}

pub(super) fn decode_immediate_reg_mov(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.is_empty() {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes to process for immediate to register",
        ));
    }

    let dst = usize::from(bytes[0] & 0x7);
    let w_bit = (bytes[0] & 0x8) != 0;
    let register_map = &REGISTER_MAPS[usize::from(w_bit)];

    let (imm, imm_len) = decode_imm(bytes, w_bit, false, 1)?;

    Ok((
        Instruction::binary("mov", register_map[dst], imm),
        imm_len + 1,
    ))
}

pub(super) fn decode_acc_mem_mov(bytes: &[u8]) -> io::Result<(Instruction, usize)> {
    if bytes.len() < 3 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes to process for accumulator to memory",
        ));
    }

    let w = (bytes[0] & 1) != 0;
    let d = (bytes[0] & 2) != 0;
    let addr = u16::from_le_bytes([bytes[1], bytes[2]]);
    let acc = ACC_REG[usize::from(w)];
    let mem = format!("[{addr}]");
    let (dst, src) = if d {
        (mem, acc.to_string())
    } else {
        (acc.to_string(), mem)
    };
    Ok((Instruction::binary("mov", dst, src), 3))
}
