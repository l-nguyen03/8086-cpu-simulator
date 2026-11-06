use crate::decode::Cursor;
use crate::instruction::{Instruction, Operand, Operation, SEGMENT_REGISTER, WORD_REGISTER, Width};
use std::fmt;

#[repr(u8)]
#[derive(Clone, Copy)]
enum Flag {
    Carry = 0,
    Parity = 2,
    Auxiliary = 4,
    Zero = 6,
    Sign = 7,
    Trap = 8,
    Interrupt = 9,
    Direction = 10,
    Overflow = 11,
}

impl Flag {
    const fn as_mask(self) -> u16 {
        match self {
            Flag::Carry => 1 << Flag::Carry as u16,
            Flag::Parity => 1 << Flag::Parity as u16,
            Flag::Auxiliary => 1 << Flag::Auxiliary as u16,
            Flag::Zero => 1 << Flag::Zero as u16,
            Flag::Sign => 1 << Flag::Sign as u16,
            Flag::Trap => 1 << Flag::Trap as u16,
            Flag::Interrupt => 1 << Flag::Interrupt as u16,
            Flag::Direction => 1 << Flag::Direction as u16,
            Flag::Overflow => 1 << Flag::Overflow as u16,
        }
    }

    const fn get_name(self) -> &'static str {
        match self {
            Flag::Carry => "Carry",
            Flag::Parity => "Parity",
            Flag::Auxiliary => "Auxiliary",
            Flag::Zero => "Zero",
            Flag::Sign => "Sign",
            Flag::Trap => "Trap",
            Flag::Interrupt => "Interrupt",
            Flag::Direction => "Direction",
            Flag::Overflow => "Overflow",
        }
    }
}

#[derive(Clone, Copy)]
struct ArithmeticResult {
    dst: u16,
    src: u16,
    value: u16,
    is_add: bool,
}

fn get_byte_shift(index: usize) -> u8 {
    ((index & 0b100) << 1) as u8
}

fn get_parity(mut value: u16) -> u16 {
    // Take only the lower byte for parity calculation
    value = value & 0x00FF;
    value = value ^ (value >> 1);
    value = value ^ (value >> 2);
    value = value ^ (value >> 4);
    value = value ^ (value >> 8);
    (value & 1u16) ^ 1u16
}

fn get_overflow(result: ArithmeticResult) -> u16 {
    if result.is_add {
        return (((result.value ^ result.dst) & (result.value ^ result.src)) & 0x8000 == 0x8000)
            as u16;
    }

    (((result.dst ^ result.src) & (result.value ^ result.dst)) & 0x8000 == 0x8000) as u16
}

fn get_carry(result: ArithmeticResult) -> u16 {
    if result.is_add {
        return (result.value < result.dst) as u16;
    }

    (result.value > result.dst) as u16
}

const CX: usize = 1;

#[derive(Default, Clone)]
pub struct Cpu {
    registers: [u16; 8],
    segment_registers: [u16; 4],
    flags_register: u16,
}

impl Cpu {
    const FLAGS: [Flag; 9] = [
        Flag::Carry,
        Flag::Parity,
        Flag::Auxiliary,
        Flag::Zero,
        Flag::Sign,
        Flag::Trap,
        Flag::Interrupt,
        Flag::Direction,
        Flag::Overflow,
    ];

    pub fn execute(&mut self, cursor: &mut Cursor, instruction: Instruction) {
        match instruction.src {
            Some(src) => self.execute_binary(instruction.mnemonic, src, instruction.dst),
            None => self.execute_control_flow(cursor, instruction.mnemonic, instruction.dst),
        }
    }

    fn execute_binary(&mut self, instruction: Operation, src: Operand, dst: Operand) {
        match instruction {
            Operation::Mov => self.write(dst, self.read(src)),
            Operation::Add => self.add(dst, src),
            Operation::Sub => self.sub(dst, src),
            Operation::Cmp => self.cmp(dst, src),
            _ => unimplemented!(),
        }
    }

    fn execute_control_flow(
        &mut self,
        cursor: &mut Cursor,
        instruction: Operation,
        offset: Operand,
    ) {
        let Operand::Imm(offset) = offset else {
            panic!("Invalid offset to control flow")
        };

        let jump = match instruction {
            Operation::Jo => self.flag(Flag::Overflow),
            Operation::Jno => !self.flag(Flag::Overflow),
            Operation::Jb => self.flag(Flag::Carry),
            Operation::Jnb => !self.flag(Flag::Carry),
            Operation::Je => self.flag(Flag::Zero),
            Operation::Jne => !self.flag(Flag::Zero),
            Operation::Jbe => self.flag(Flag::Carry) | self.flag(Flag::Zero),
            Operation::Jnbe => !(self.flag(Flag::Carry) | self.flag(Flag::Zero)),
            Operation::Js => self.flag(Flag::Sign),
            Operation::Jns => !self.flag(Flag::Sign),
            Operation::Jp => self.flag(Flag::Parity),
            Operation::Jnp => !self.flag(Flag::Parity),
            Operation::Jl => self.flag(Flag::Sign) != self.flag(Flag::Overflow),
            Operation::Jnl => self.flag(Flag::Sign) == self.flag(Flag::Overflow),
            Operation::Jle => {
                self.flag(Flag::Zero) | (self.flag(Flag::Sign) != self.flag(Flag::Overflow))
            }
            Operation::Jnle => {
                !self.flag(Flag::Zero) & (self.flag(Flag::Sign) == self.flag(Flag::Overflow))
            }
            Operation::Loopnz => (self.decrement_cx() != 0) & !self.flag(Flag::Zero),
            Operation::Loopz => (self.decrement_cx() != 0) & self.flag(Flag::Zero),
            Operation::Loop => self.decrement_cx() != 0,
            Operation::Jcxz => self.registers[CX] == 0,
            _ => unreachable!("{instruction} is not a control flow instruction"),
        };

        cursor.move_cursor_by(jump as i16 * offset);
    }

    fn flag(&self, flag: Flag) -> bool {
        self.flags_register & flag.as_mask() != 0
    }

    fn decrement_cx(&mut self) -> u16 {
        self.registers[CX] = self.registers[CX].wrapping_sub(1);
        self.registers[CX]
    }

    fn read(&self, operand: Operand) -> u16 {
        match operand {
            Operand::GeneralRegister { index, width } => self.read_register(index, width),
            Operand::SegmentRegister { index } => self.segment_registers[index],
            Operand::Imm(value) => value as u16,
            _ => unimplemented!(),
        }
    }

    fn write(&mut self, operand: Operand, value: u16) {
        match operand {
            Operand::GeneralRegister { index, width } => self.write_register(index, width, value),
            Operand::SegmentRegister { index } => self.segment_registers[index] = value,
            _ => unimplemented!(),
        }
    }

    fn add(&mut self, dst: Operand, src: Operand) {
        let dst_value = self.read(dst);
        let src_value = self.read(src);
        let result = ArithmeticResult {
            dst: dst_value,
            src: src_value,
            value: dst_value.wrapping_add(src_value),
            is_add: true,
        };

        self.write(dst, result.value);
        self.write_flags(result);
    }

    fn sub(&mut self, dst: Operand, src: Operand) {
        let dst_value = self.read(dst);
        let src_value = self.read(src);
        let result = ArithmeticResult {
            dst: dst_value,
            src: src_value,
            value: dst_value.wrapping_sub(src_value),
            is_add: false,
        };

        self.write(dst, result.value);
        self.write_flags(result);
    }

    fn cmp(&mut self, dst: Operand, src: Operand) {
        let dst_value = self.read(dst);
        let src_value = self.read(src);
        let result = ArithmeticResult {
            dst: dst_value,
            src: src_value,
            value: dst_value.wrapping_sub(src_value),
            is_add: false,
        };

        self.write_flags(result);
    }

    fn write_flags(&mut self, result: ArithmeticResult) {
        let overflow = get_overflow(result);
        let carry = get_carry(result);

        let parity = get_parity(result.value);
        let zero = (result.value == 0) as u16;
        let sign = ((result.value & 0x8000) == 0x8000) as u16;
        let auxiliary_carry = (((result.value ^ result.src ^ result.dst) & 0x10) == 0x10) as u16;

        self.flags_register = (self.flags_register
            & !(Flag::Carry.as_mask()
                | Flag::Parity.as_mask()
                | Flag::Zero.as_mask()
                | Flag::Sign.as_mask()
                | Flag::Overflow.as_mask()
                | Flag::Auxiliary.as_mask()))
            | (carry * Flag::Carry.as_mask())
            | (parity * Flag::Parity.as_mask())
            | (zero * Flag::Zero.as_mask())
            | (sign * Flag::Sign.as_mask())
            | (overflow * Flag::Overflow.as_mask())
            | (auxiliary_carry * Flag::Auxiliary.as_mask());
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

        for index in 0..self.segment_registers.len() {
            writeln!(
                f,
                "{}: {:#06X}",
                SEGMENT_REGISTER[index], self.segment_registers[index]
            )?
        }

        for flag in Cpu::FLAGS {
            writeln!(
                f,
                "{}: {:1}",
                flag.get_name(),
                (self.flags_register & flag.as_mask()) >> (flag as u16)
            )?
        }

        Ok(())
    }
}
