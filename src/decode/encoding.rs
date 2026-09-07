use std::io;

const BYTE_REGISTER: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
const WORD_REGISTER: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];
pub(super) const REGISTER_MAPS: [[&str; 8]; 2] = [BYTE_REGISTER, WORD_REGISTER];

const MOD_SHIFT: u8 = 6;

const EFFECTIVE_ADDRESS: [&str; 8] = [
    "bx + si", "bx + di", "bp + si", "bp + di", "si", "di", "bp", "bx",
];

pub(super) const ACC_REG: [&str; 2] = ["al", "ax"];

const DW_MASK: u8 = 0x3;

pub(super) struct InstructionFields {
    pub(super) reg_field: usize,
    pub(super) rm_field: usize,
    pub(super) mode: u8,
    pub(super) d_bit: bool,
    pub(super) w_bit: bool,
}

pub(super) fn format_error(error_kind: io::ErrorKind, msg: &str) -> io::Error {
    io::Error::new(error_kind, msg)
}

fn decompose_registers(byte: u8) -> (usize, usize) {
    let rm = usize::from(byte & 0x7);
    let reg = usize::from((byte >> 3) & 0x7);
    (reg, rm)
}

pub(super) fn decompose_instruction_field(bytes: &[u8]) -> io::Result<InstructionFields> {
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

pub(super) fn decode_rm_field(
    bytes: &[u8],
    fields: &InstructionFields,
) -> io::Result<(String, usize)> {
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

pub(super) fn decode_imm(
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

pub(super) fn with_size_prefix(operand: String, is_memory: bool, w_bit: bool) -> String {
    if !is_memory {
        return operand;
    }
    let size = if w_bit { "word" } else { "byte" };
    format!("{size} {operand}")
}
