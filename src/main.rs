#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::io::prelude::*;
use std::fs::File;
use std::path::Path; 

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};


// TODO:
// * test Value::new


#[cfg(test)]
mod tests {
    #[test]
    fn load_program_from_file() -> Result<(), ()>{
        use super::VM;

        let f = "challenge.bin";
        let mut vm = VM::new();
        vm.load_program_from_file(f)
    }

    #[test]
    fn load_program_from_mem() -> Result<(), ()>{
        use super::VM;

        let mut vm = VM::new();
        let program = vec![9,32768,32769,4,19,32768];
        vm.load_program_from_mem(program);

        Ok(())
    }
}


#[derive(Copy, Clone, Debug)]
enum Value {
    Number(u16),
    Register(usize),
    Invalid,
}
impl Value {
    fn new(v: u16) -> Self {
        match v {
            0 ... 32767 => Value::Number(v),
            32768 ... 32775 => Value::Register((v - 32768) as usize),
            32776 ... 65535 => Value::Invalid,
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Opcode {
    Halt,
    Out(Value),
    Noop,
}

const MEM_SIZE: usize = 32768;

#[derive(Clone)]
struct VM {
    //memory: Vec<u16>,
    memory: [u16; MEM_SIZE],
    registers: [u16; 8],
    stack: Vec<u16>,
    /// Instruction pointer (next instruction)
    ip: usize,
    /// Program counter (number of executed instructions)
    pc: usize
}


impl VM {
    fn new() -> Self {
        VM {
            //memory: Vec::new(),
            memory: [0u16; MEM_SIZE],
            registers: [0u16; 8],
            stack: Vec::new(),
            ip: 0,
            pc: 0,
        }
    }

    fn load_program_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ()> {
        let mut f = File::open(path).map_err(|_| ())?;
        let mut buff = Vec::new();

        f.read_to_end(&mut buff).map_err(|_| ())?;

        let data: Vec<_> = buff
            .chunks(2)
            .map(|x| {
                let pair = x.iter().map(|x| *x).collect::<Vec<u8>>();
                LittleEndian::read_u16(&pair)
            }
            )
            .collect();

        println!("data len={}", data.len());
        if data.len() >= MEM_SIZE {
            panic!("File is too big");
        }
        for i in 0..data.len() {
            self.memory[i] = data[i];
        }

        Ok(())
    }

    fn load_program_from_mem(&mut self, program: &[u16]) {
        for i in 0..program.len() {
            self.memory[i] = program[i];
        }
    }

    fn run(&mut self) {
        while !self.step() {}
    }

    fn step(&mut self) -> bool {
        let (instruction, size) = self.fetch();

        let next_instruction_ptr = self.ip + size;
        let must_exit = self.execute(&instruction, next_instruction_ptr);

        self.pc += 1;

        must_exit
    }

    fn fetch(&self) -> (Opcode, usize) {
      let instr_type = self.memory[self.ip];

      match instr_type {
          0 => (Opcode::Halt, 1),
          //1 => unimplemented!("{}", instr_type),
          //2 => unimplemented!("{}", instr_type),
          //3 => unimplemented!("{}", instr_type),
          //4 => unimplemented!("{}", instr_type),
          //5 => unimplemented!("{}", instr_type),
          //6 => unimplemented!("{}", instr_type),
          //7 => unimplemented!("{}", instr_type),
          //8 => unimplemented!("{}", instr_type),
          //9 => unimplemented!("{}", instr_type),
          //11 => unimplemented!("{}", instr_type),
          //12 => unimplemented!("{}", instr_type),
          //13 => unimplemented!("{}", instr_type),
          //14 => unimplemented!("{}", instr_type),
          //15 => unimplemented!("{}", instr_type),
          //16 => unimplemented!("{}", instr_type),
          //17 => unimplemented!("{}", instr_type),
          //18 => unimplemented!("{}", instr_type),
          19 => (Opcode::Out(Value::new(self.memory[self.ip + 1])), 2),
          //20 => unimplemented!("{}", instr_type),
          21 => (Opcode::Noop, 1),
          //_ => unreachable!(),
          _ => (Opcode::Noop, 1),
      }
    }

    fn execute(&mut self, instruction: &Opcode, next_instruction_ptr: usize) -> bool {
        //println!("{:?}", instruction);

        self.ip = next_instruction_ptr;

        let mut ret = false;
        match instruction {
            Opcode::Halt => ret = true,
            Opcode::Noop => (),
            Opcode::Out(x) => {
                let c = match x {
                    Value::Number(x) => *x,
                    Value::Register(x) => self.registers[*x],
                    Value::Invalid => panic!("Out: invalid number"),
                };

                print!("{}", c as u8 as char);
            }
            _ => (),
        }

        ret
    }
}


fn main() {
    let mut vm = VM::new();
    vm.load_program_from_file("challenge.bin").unwrap();

    vm.run();

}
