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

const DW_MASK: u8 = 0x3;
const OP_CODE_MASK: u8 = 0b1111_1100;

struct InstructionFields {
    reg_field: usize,
    rm_field: usize,
    mode: u8,
    d_bit: bool,
    w_bit: bool,
}

fn eof_error(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, msg)
}

fn decompose_registers(byte: u8) -> (usize, usize) {
    let rm = usize::from(byte & 0x7);
    let reg = usize::from((byte >> 3) & 0x7);
    (reg, rm)
}

fn decompose_instruction_field(bytes: &[u8]) -> io::Result<InstructionFields> {
    if bytes.len() < 2 {
        return Err(eof_error("Not enough bytes to decode instruction"));
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
            let disp = bytes
                .get(2)
                .copied()
                .ok_or_else(|| eof_error("truncated displacement"))?;
            Ok((
                format_effective_address(fields.rm_field, Some(i16::from(disp as i8))),
                1,
            ))
        }
        0b10 => {
            if bytes.len() < 4 {
                return Err(eof_error("truncated displacement"));
            }
            let disp = u16::from_le_bytes([bytes[2], bytes[3]]) as i16;
            Ok((format_effective_address(fields.rm_field, Some(disp)), 2))
        }
        0b11 => Ok((register_map[fields.rm_field].to_string(), 0)),
        0b00 => {
            if fields.rm_field == 0b110 {
                if bytes.len() < 4 {
                    return Err(eof_error("truncated displacement"));
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

fn decode_reg_to_reg_mov(bytes: &[u8]) -> io::Result<([String; 2], usize)> {
    let fields = decompose_instruction_field(bytes)?;
    let register_map = &REGISTER_MAPS[usize::from(fields.w_bit)];
    let reg = register_map[fields.reg_field].to_string();
    let (rm, disp_len) = decode_rm_field(bytes, &fields)?;
    let (dst, src) = if fields.d_bit { (reg, rm) } else { (rm, reg) };
    Ok(([dst, src], 2 + disp_len))
}

fn decode_immediate_rm_mov(bytes: &[u8]) -> io::Result<([String; 2], usize)> {
    let fields = decompose_instruction_field(bytes)?;
    if fields.reg_field != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected MOV immediate-to-r/m (/0)",
        ));
    }

    let (dst, disp_len) = decode_rm_field(bytes, &fields)?;
    let imm_at = 2 + disp_len;
    let (src, imm_len) = if fields.w_bit {
        if bytes.len() < imm_at + 2 {
            return Err(eof_error(
                "Not enough bytes to process for immediate to register/memory",
            ));
        }
        let imm = u16::from_le_bytes([bytes[imm_at], bytes[imm_at + 1]]) as i16;
        (imm.to_string(), 2)
    } else {
        let imm = bytes.get(imm_at).copied().ok_or_else(|| {
            eof_error("Not enough bytes to process for immediate to register/memory")
        })?;
        ((imm as i8).to_string(), 1)
    };
    Ok(([dst, src], imm_at + imm_len))
}

fn decode_immediate_reg_mov(bytes: &[u8]) -> io::Result<([String; 2], usize)> {
    if bytes.is_empty() {
        return Err(eof_error(
            "Not enough bytes to process for immediate to register",
        ));
    }

    let dst = usize::from(bytes[0] & 0x7);
    let w_bit = (bytes[0] & 0x8) != 0;
    let register_map = &REGISTER_MAPS[usize::from(w_bit)];

    if w_bit {
        if bytes.len() < 3 {
            return Err(eof_error(
                "Not enough bytes to process for immediate to register",
            ));
        }
        let imm = u16::from_le_bytes([bytes[1], bytes[2]]) as i16;
        Ok(([register_map[dst].to_string(), imm.to_string()], 3))
    } else {
        if bytes.len() < 2 {
            return Err(eof_error(
                "Not enough bytes to process for immediate to register",
            ));
        }
        Ok((
            [register_map[dst].to_string(), (bytes[1] as i8).to_string()],
            2,
        ))
    }
}

fn decode_acc_mem_mov(bytes: &[u8]) -> io::Result<([String; 2], usize)> {
    if bytes.len() < 3 {
        return Err(eof_error(
            "Not enough bytes to process for accumulator to memory",
        ));
    }

    let w = (bytes[0] & 1) != 0;
    let d = (bytes[0] & 2) != 0;
    let addr = u16::from_le_bytes([bytes[1], bytes[2]]);
    let acc = if w { "ax" } else { "al" };
    let mem = format!("[{addr}]");
    let (dst, src) = if d {
        (mem, acc.to_string())
    } else {
        (acc.to_string(), mem)
    };
    Ok(([dst, src], 3))
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
        let ([dst_register, src_register], bytes_read) = match opcode & OP_CODE_MASK {
            0b1000_1000 => decode_reg_to_reg_mov(remaining)?,
            0b1010_0000 => decode_acc_mem_mov(remaining)?,
            _ if (opcode & 0b1111_1110) == 0b1100_0110 => decode_immediate_rm_mov(remaining)?,
            _ if (opcode & 0b1111_0000) == 0b1011_0000 => decode_immediate_reg_mov(remaining)?,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown opcode {opcode:#04x}"),
                ));
            }
        };

        write!(writer, "mov {dst_register}, {src_register}\n")?;
        processed_bytes += bytes_read;
    }

    Ok(())
}
