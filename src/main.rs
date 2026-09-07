use std::env;
use std::fs;
use std::io::{self, BufWriter, Write};

mod decode;
mod instruction;

fn parse_args() -> (String, String) {
    let mut args = env::args();
    let program = args.next().unwrap();

    let Some(in_path) = args.next() else {
        eprintln!("Usage: {program} <binary-file> [out-file]");
        std::process::exit(1);
    };

    let out_path = args.next().unwrap_or_else(|| "assembled.asm".into());
    (in_path, out_path)
}

fn main() -> io::Result<()> {
    let (in_path, out_path) = parse_args();
    let bytes = fs::read(in_path)?;
    let mut writer = BufWriter::new(fs::File::create(&out_path)?);
    writer.write_all(b"bits 16\n")?;

    let mut cur = decode::Cursor::new(&bytes);
    while !cur.is_empty() {
        let instruction = decode::instruction(&mut cur)?;
        writeln!(writer, "{instruction}")?;
    }

    Ok(())
}
