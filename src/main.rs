mod cli_args;
mod cpu;
mod decode;
mod instruction;

use cli_args::Args;
use cpu::Cpu;
use instruction::Instruction;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

enum Output {
    File(File),
    Stdout(io::Stdout),
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::File(f) => f.write(buf),
            Output::Stdout(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::File(f) => f.flush(),
            Output::Stdout(s) => s.flush(),
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let bytes = fs::read(args.in_path)?;

    let outpath = match (args.exec, args.out_path) {
        (_, Some(path)) => Output::File(fs::File::create(path)?),
        (true, None) => Output::Stdout(io::stdout()),
        (false, None) => Output::File(fs::File::create("assembled.asm".to_string())?),
    };

    let mut buf_writer = BufWriter::new(outpath);
    buf_writer.write_all(b"bits 16\n")?;

    let mut cpu = Cpu::default();
    let mut instruction_vec: Vec<Instruction> = Vec::new();

    let mut cur = decode::Cursor::new(&bytes);
    while !cur.is_empty() {
        let instruction = decode::instruction(&mut cur)?;
        writeln!(buf_writer, "{instruction}")?;
        instruction_vec.push(instruction);
    }

    if args.exec {
        for instruction in instruction_vec {
            cpu.execute(instruction)
        }

        writeln!(buf_writer, "Final registers' state:")?;
        writeln!(buf_writer, "{cpu}")?;
    }

    Ok(())
}
