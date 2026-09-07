use super::encoding::{acc, read_imm, read_modrm, read_rm, register, with_size};
use super::{Cursor, DecodeError};
use crate::instruction::{Instruction, Operand};

pub(super) fn alu_op(op: u8) -> Result<&'static str, DecodeError> {
    match op & 7 {
        0 => Ok("add"),
        5 => Ok("sub"),
        7 => Ok("cmp"),
        other => Err(DecodeError::UnsupportedAlu(other)),
    }
}

fn order_operands(d: bool, reg: Operand, rm: Operand) -> (Operand, Operand) {
    if d { (reg, rm) } else { (rm, reg) }
}

pub(super) fn decode_rm_reg(
    cur: &mut Cursor<'_>,
    mnemonic: &'static str,
    d: bool,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let modrm = read_modrm(cur)?;
    let reg = Operand::Register(register(w, modrm.reg));
    let rm = read_rm(cur, &modrm, w)?;
    let (dst, src) = order_operands(d, reg, rm);
    Ok(Instruction::binary(mnemonic, dst, src))
}

pub(super) fn decode_imm_rm(
    cur: &mut Cursor<'_>,
    w: bool,
    sign_extend: bool,
    mnemonic: impl FnOnce(u8) -> Result<&'static str, DecodeError>,
) -> Result<Instruction, DecodeError> {
    let modrm = read_modrm(cur)?;
    let mnemonic = mnemonic(modrm.reg)?;
    let dst = read_rm(cur, &modrm, w)?;
    let dst = if modrm.mode != 0b11 {
        with_size(dst, w)
    } else {
        dst
    };
    let imm = read_imm(cur, w, sign_extend)?;
    Ok(Instruction::binary(mnemonic, dst, Operand::Imm(imm)))
}

pub(super) fn decode_imm_acc(
    cur: &mut Cursor<'_>,
    mnemonic: &'static str,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let imm = read_imm(cur, w, false)?;
    Ok(Instruction::binary(
        mnemonic,
        Operand::Register(acc(w)),
        Operand::Imm(imm),
    ))
}

pub(super) fn decode_imm_reg(
    cur: &mut Cursor<'_>,
    w: bool,
    reg: u8,
) -> Result<Instruction, DecodeError> {
    let imm = read_imm(cur, w, false)?;
    Ok(Instruction::binary(
        "mov",
        Operand::Register(register(w, reg)),
        Operand::Imm(imm),
    ))
}

pub(super) fn decode_acc_mem(
    cur: &mut Cursor<'_>,
    d: bool,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let addr = cur.u16()?;
    let acc = Operand::Register(acc(w));
    let mem = Operand::Memory {
        ea: None,
        disp: addr as i16,
        direct: true,
        size: None,
    };
    let (dst, src) = if d { (mem, acc) } else { (acc, mem) };
    Ok(Instruction::binary("mov", dst, src))
}

pub(super) fn decode_jump(
    cur: &mut Cursor<'_>,
    mnemonic: &'static str,
) -> Result<Instruction, DecodeError> {
    Ok(Instruction::unary(
        mnemonic,
        Operand::Imm(i16::from(cur.i8()?)),
    ))
}
