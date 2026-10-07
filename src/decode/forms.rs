use super::encoding::{acc, read_imm, read_modrm, read_rm, register, with_size};
use super::{Cursor, DecodeError};
use crate::instruction::{Instruction, Operand, Operation};

const JCC: [Operation; 16] = [
    Operation::Jo,
    Operation::Jno,
    Operation::Jb,
    Operation::Jnb,
    Operation::Je,
    Operation::Jne,
    Operation::Jbe,
    Operation::Jnbe,
    Operation::Js,
    Operation::Jns,
    Operation::Jp,
    Operation::Jnp,
    Operation::Jl,
    Operation::Jnl,
    Operation::Jle,
    Operation::Jnle,
];

const LOOP: [Operation; 4] = [
    Operation::Loopnz,
    Operation::Loopz,
    Operation::Loop,
    Operation::Jcxz,
];

pub(super) fn alu_op(op: u8) -> Result<Operation, DecodeError> {
    match op & 7 {
        0 => Ok(Operation::Add),
        5 => Ok(Operation::Sub),
        7 => Ok(Operation::Cmp),
        other => Err(DecodeError::UnsupportedAlu(other)),
    }
}

fn order_operands(d: bool, reg: Operand, rm: Operand) -> (Operand, Operand) {
    if d { (reg, rm) } else { (rm, reg) }
}

pub(super) fn decode_rm_reg(
    cur: &mut Cursor<'_>,
    mnemonic: Operation,
    d: bool,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let modrm = read_modrm(cur)?;
    let reg = register(w, modrm.reg);
    let rm = read_rm(cur, &modrm, w)?;
    let (dst, src) = order_operands(d, reg, rm);
    Ok(Instruction::binary(mnemonic, dst, src))
}

pub(super) fn decode_imm_rm(
    cur: &mut Cursor<'_>,
    w: bool,
    sign_extend: bool,
    mnemonic: impl FnOnce(u8) -> Result<Operation, DecodeError>,
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
    mnemonic: Operation,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let imm = read_imm(cur, w, false)?;
    Ok(Instruction::binary(mnemonic, acc(w), Operand::Imm(imm)))
}

pub(super) fn decode_imm_reg(
    cur: &mut Cursor<'_>,
    w: bool,
    reg: u8,
) -> Result<Instruction, DecodeError> {
    let imm = read_imm(cur, w, false)?;
    Ok(Instruction::binary(
        Operation::Mov,
        register(w, reg),
        Operand::Imm(imm),
    ))
}

pub(super) fn decode_acc_mem(
    cur: &mut Cursor<'_>,
    d: bool,
    w: bool,
) -> Result<Instruction, DecodeError> {
    let addr = cur.u16()?;
    let acc = acc(w);
    let mem = Operand::Memory {
        ea: None,
        disp: addr as i16,
        direct: true,
        size: None,
    };
    let (dst, src) = if d { (mem, acc) } else { (acc, mem) };
    Ok(Instruction::binary(Operation::Mov, dst, src))
}

pub(super) fn decode_jump(
    cur: &mut Cursor<'_>,
    op: u8,
    is_loop: bool,
) -> Result<Instruction, DecodeError> {
    let mnemonic = if is_loop {
        JCC[usize::from(op & 0xF)]
    } else {
        LOOP[usize::from(op & 0x3)]
    };
    Ok(Instruction::unary(
        mnemonic,
        Operand::Imm(i16::from(cur.i8()?)),
    ))
}
