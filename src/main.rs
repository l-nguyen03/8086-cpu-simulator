use std::env;
use std::fs;
use std::io::{self, BufWriter, Write};

const BYTE_REGISTER: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
const WORD_REGISTER: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];
const REGISTER_MAPS: [[&str; 8]; 2] = [BYTE_REGISTER, WORD_REGISTER];

const MOD_SHIFT: u8 = 6;

const EFFECTIVE_ADDRESS: [&str; 8] = [
    "bx + si", "bx + di", "bp + si", "bp + di", "si", "di", "bp", "bx",
];

const ACC_REG: [&str; 2] = ["al", "ax"];

const COND_JUMP_LOOKUP: [&str; 16] = [
    "jo", "jno", "jb", "jnb", "je", "jne", "jbe", "jnbe", "js", "jns", "jp", "jnp", "jl", "jnl",
    "jle", "jnle",
];

const LOOP_JMP_LOOKUP: [&str; 4] = ["loopnz", "loopz", "loop", "jcxz"];

const DW_MASK: u8 = 0x3;
const OP_CODE_MASK: u8 = 0b1111_1100;

struct InstructionFields {
    reg_field: usize,
    rm_field: usize,
    mode: u8,
    d_bit: bool,
    w_bit: bool,
}

fn format_error(error_kind: io::ErrorKind, msg: &str) -> io::Error {
    io::Error::new(error_kind, msg)
}

fn decompose_registers(byte: u8) -> (usize, usize) {
    let rm = usize::from(byte & 0x7);
    let reg = usize::from((byte >> 3) & 0x7);
    (reg, rm)
}

fn decompose_instruction_field(bytes: &[u8]) -> io::Result<InstructionFields> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes to decode instruction",
        ));
    }
    let (reg_field, rm_field) = decompose_registers(bytes[1]);
    let dw_field = bytes[0] & DW_MASK;
    let mode = bytes[1] >> MOD_SHIFT;
    Ok(InstructionFields {
        reg_field,
        rm_field,
        mode,
        d_bit: (dw_field & 0x2) != 0,
        w_bit: (dw_field & 0x1) != 0,
    })
}

fn format_effective_address(rm_field: usize, disp: Option<i16>) -> String {
    let ea = EFFECTIVE_ADDRESS[rm_field];
    match disp {
        None | Some(0) => format!("[{ea}]"),
        Some(d) if d < 0 => format!("[{ea} - {}]", d.unsigned_abs()),
        Some(d) => format!("[{ea} + {d}]"),
    }
}

fn decode_rm_field(bytes: &[u8], fields: &InstructionFields) -> io::Result<(String, usize)> {
    let register_map = &REGISTER_MAPS[usize::from(fields.w_bit)];

    match fields.mode {
        0b01 => {
            let disp = bytes.get(2).copied().ok_or_else(|| {
                format_error(io::ErrorKind::UnexpectedEof, "truncated displacement")
            })?;
            Ok((
                format_effective_address(fields.rm_field, Some(i16::from(disp as i8))),
                1,
            ))
        }
        0b10 => {
            if bytes.len() < 4 {
                return Err(format_error(
                    io::ErrorKind::UnexpectedEof,
                    "truncated displacement",
                ));
            }
            let disp = u16::from_le_bytes([bytes[2], bytes[3]]) as i16;
            Ok((format_effective_address(fields.rm_field, Some(disp)), 2))
        }
        0b11 => Ok((register_map[fields.rm_field].to_string(), 0)),
        0b00 => {
            if fields.rm_field == 0b110 {
                if bytes.len() < 4 {
                    return Err(format_error(
                        io::ErrorKind::UnexpectedEof,
                        "truncated displacement",
                    ));
                }
                let addr = u16::from_le_bytes([bytes[2], bytes[3]]);
                Ok((format!("[{addr}]"), 2))
            } else {
                Ok((format_effective_address(fields.rm_field, None), 0))
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid mod field {:#04x}", fields.mode),
        )),
    }
}

fn decode_arithmetic_opcode(byte: u8) -> io::Result<String> {
    const ARITHMETIC_OP_MASK: u8 = 0b0011_1000;
    let op_code = byte & ARITHMETIC_OP_MASK;
    match op_code {
        0b0000_0000 => Ok("add".to_string()),
        0b0010_1000 => Ok("sub".to_string()),
        0b0011_1000 => Ok("cmp".to_string()),
        _ => {
            return Err(format_error(
                io::ErrorKind::InvalidData,
                "unknown op code for arithmetic immediate to/from acc",
            ));
        }
    }
}

fn decode_imm(
    bytes: &[u8],
    is_wide: bool,
    is_sign_extended: bool,
    start: usize,
) -> io::Result<(String, usize)> {
    if bytes.len() <= is_wide as usize * (1 - is_sign_extended as usize) + start {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes to process for immediate to register",
        ));
    }

    // s/w select encoding width; immediates are always printed signed.
    let sw = is_wide as u8 | ((is_sign_extended as u8) << 1);
    match sw {
        0b00 | 0b10 => Ok(((bytes[start] as i8).to_string(), 1)),
        0b11 => Ok(((bytes[start] as i8 as i16).to_string(), 1)),
        0b01 => Ok((
            i16::from_le_bytes([bytes[start], bytes[start + 1]]).to_string(),
            2,
        )),
        _ => unreachable!(),
    }
}

fn with_size_prefix(operand: String, is_memory: bool, w_bit: bool) -> String {
    if !is_memory {
        return operand;
    }
    let size = if w_bit { "word" } else { "byte" };
    format!("{size} {operand}")
}

fn decode_reg_to_reg_mov(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
    let fields = decompose_instruction_field(bytes)?;
    let register_map = &REGISTER_MAPS[usize::from(fields.w_bit)];
    let reg = register_map[fields.reg_field].to_string();
    let (rm, disp_len) = decode_rm_field(bytes, &fields)?;
    let (dst, src) = if fields.d_bit { (reg, rm) } else { (rm, reg) };
    Ok((["mov".to_string(), dst, src], 2 + disp_len))
}

fn decode_immediate_rm_mov(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
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
    Ok((["mov".to_string(), dst, imm], imm_len + disp_len + 2))
}

fn decode_immediate_reg_mov(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
    if bytes.is_empty() {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes to process for immediate to register",
        ));
    }

    let dst = usize::from(bytes[0] & 0x7);
    let w_bit = (bytes[0] & 0x8) != 0;
    let register_map = &REGISTER_MAPS[usize::from(w_bit)];
    let op_name = "mov".to_string();

    let (imm, imm_len) = decode_imm(bytes, w_bit, false, 1)?;

    Ok((
        [op_name, register_map[dst].to_string(), imm.to_string()],
        imm_len + 1,
    ))
}

fn decode_acc_mem_mov(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
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
    Ok((["mov".to_string(), dst, src], 3))
}

fn decode_arithmetic_immediate_and_acc(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
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

    Ok((
        [op_name.to_string(), acc.to_string(), imm.to_string()],
        imm_len + 1,
    ))
}

fn decode_arithmetic_immediate_and_rm(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
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

    Ok(([op_name.to_string(), dst, imm], imm_len + disp_len + 2))
}

fn decode_arithmetic_rm_and_reg(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
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
    Ok(([op_name.to_string(), dst, src], disp_len + 2))
}

fn decode_cond_jmp(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for conditional jump",
        ));
    }

    let op = COND_JUMP_LOOKUP[(bytes[0] & 0b0000_1111) as usize];
    Ok((
        [op.to_string(), (bytes[1] as i8).to_string(), "".to_string()],
        2,
    ))
}

fn decode_loop(bytes: &[u8]) -> io::Result<([String; 3], usize)> {
    if bytes.len() < 2 {
        return Err(format_error(
            io::ErrorKind::UnexpectedEof,
            "Not enough bytes for loop instructions",
        ));
    }

    let op = LOOP_JMP_LOOKUP[(bytes[0] & 0b0000_1111) as usize];
    Ok((
        [op.to_string(), (bytes[1] as i8).to_string(), "".to_string()],
        2,
    ))
}

fn main() -> io::Result<()> {
    let mut args = env::args();
    let program = args.next().unwrap();

    let Some(in_path) = args.next() else {
        eprintln!("Usage: {program} <binary-file> [out-file]");
        std::process::exit(1);
    };

    let out_path = args.next().unwrap_or_else(|| "assembled.asm".into());

    let instructions = fs::read(in_path)?;
    let mut processed_bytes = 0;
    let total_bytes = instructions.len();

    let out = fs::File::create(&out_path)?;
    let mut writer = BufWriter::new(out);
    writer.write_all(b"bits 16\n")?;

    while processed_bytes < total_bytes {
        let remaining = &instructions[processed_bytes..];
        let opcode = remaining[0];
        let ([op_name, dst_register, src_register], bytes_read) = match opcode & OP_CODE_MASK {
            0b1000_1000 => decode_reg_to_reg_mov(remaining)?,
            0b1010_0000 => decode_acc_mem_mov(remaining)?,
            _ if (opcode & 0b1111_1110) == 0b1100_0110 => decode_immediate_rm_mov(remaining)?,
            _ if (opcode & 0b1111_0000) == 0b1011_0000 => decode_immediate_reg_mov(remaining)?,
            _ if (opcode & 0b1100_0100) == 0b0000_0100 => {
                decode_arithmetic_immediate_and_acc(remaining)?
            }
            0b1000_0000 => decode_arithmetic_immediate_and_rm(remaining)?,
            _ if (opcode & 0b1100_0100) == 0b0000_0000 => decode_arithmetic_rm_and_reg(remaining)?,
            _ if (opcode & 0b1111_0000 == 0b0111_0000) => decode_cond_jmp(remaining)?,
            0b1110_0000 => decode_loop(remaining)?,
            _ => {
                return Err(format_error(
                    io::ErrorKind::InvalidData,
                    &format!("unknown opcode {opcode:#04x}"),
                ));
            }
        };

        if !src_register.is_empty() {
            write!(writer, "{op_name} {dst_register}, {src_register}\n")?;
        } else {
            write!(writer, "{op_name} {dst_register}\n")?;
        }

        processed_bytes += bytes_read;
    }

    Ok(())
}
