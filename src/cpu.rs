use crate::instruction::{Instruction, Operand, Operation, WORD_REGISTER, Width};
use std::fmt;

fn get_byte_shift(index: usize) -> u8 {
    ((index & 0b100) << 1) as u8
}

#[derive(Default, Clone, Copy)]
pub struct Cpu {
    registers: [u16; 8],
}

impl Cpu {
    pub fn execute(&mut self, instruction: Instruction) {
        match instruction.src {
            Some(src) => self.execute_binary(instruction.mnemonic, src, instruction.dst),
            None => unimplemented!(),
        }
    }

    fn execute_binary(&mut self, instruction: Operation, src: Operand, dst: Operand) {
        match instruction {
            Operation::Mov => self.write(dst, self.read(src)),
            _ => unimplemented!(),
        }
    }

    fn read(&self, operand: Operand) -> u16 {
        match operand {
            Operand::Register { index, width } => self.read_register(index, width),
            Operand::Imm(value) => value as u16,
            _ => unimplemented!(),
        }
    }

    fn write(&mut self, operand: Operand, value: u16) {
        match operand {
            Operand::Register { index, width } => self.write_register(index, width, value),
            _ => unimplemented!(),
        }
    }

    fn read_register(&self, index: usize, width: Width) -> u16 {
        match width {
            Width::Word => self.registers[index],
            Width::Byte => {
                let value = self.registers[index & 0b11];
                (value >> get_byte_shift(index)) & 0xFF
            }
        }
    }

    fn write_register(&mut self, index: usize, width: Width, value: u16) {
        match width {
            Width::Word => self.registers[index] = value,
            Width::Byte => {
                let dst = &mut self.registers[index & 0b11];
                let shift = get_byte_shift(index);
                let mask = !(0xFF << shift);
                let value = (value & 0xFF) << shift;
                *dst = (*dst & mask) | value;
            }
        }
    }
}

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for index in 0..self.registers.len() {
            writeln!(
                f,
                "{}: {:#06X}",
                WORD_REGISTER[index], self.registers[index]
            )?
        }
        Ok(())
    }
}
