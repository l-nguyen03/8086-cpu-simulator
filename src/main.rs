use std::env;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::mem;

const BYTE_REGISTER: [&str; 8] = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"];
const WORD_REGISTER: [&str; 8] = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"];
const REGISTER_MAPS: [[&str; 8]; 2] = [BYTE_REGISTER, WORD_REGISTER];
const REGISTERS_MASK: u8 = 0x3F;
const DW_MASK: u8 = 0x3;

fn main() -> io::Result<()> {
    let mut args = env::args();
    let program = args.next().unwrap();

    let Some(in_path) = args.next() else {
        eprintln!("Usage: {program} <binary-file> [out-file]");
        std::process::exit(1);
    };

    let out_path = args.next().unwrap_or_else(|| "assembled.asm".into());

    // Assume always MOD = 11, i.e. only Registers no Displacment
    // Opcode is always MOV from register to register
    let instructions = fs::read(in_path)?;
    let total_bytes = instructions.len();
    let mut processed_bytes = 0;
    let mut decoded_output = Vec::<String>::with_capacity(total_bytes);

    while processed_bytes < total_bytes {
        let dw_field = instructions[processed_bytes] & DW_MASK;
        let registers = instructions[processed_bytes + 1] & REGISTERS_MASK;
        let register_map = &REGISTER_MAPS[usize::from(dw_field & 0x1)];
        let mut dst_register = register_map[usize::from(registers & 0x7)];
        let mut src_register = register_map[usize::from((registers & 0x38) >> 3)];

        if (dw_field & 0x2) == 1 {
            mem::swap(&mut dst_register, &mut src_register);
        }
        decoded_output.push(format!("mov {}, {}\n", dst_register, src_register));
        processed_bytes += 2;
    }

    let out = fs::File::create(&out_path)?;
    let mut writer = BufWriter::new(out);
    writer.write_all(b"bits 16\n")?;
    for instruction in &decoded_output {
        writer.write_all(instruction.as_bytes())?;
    }
    Ok(())
}
