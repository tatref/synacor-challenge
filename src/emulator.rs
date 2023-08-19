use std::{collections::VecDeque, fmt, fs::File, io::Read, path::Path};

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Value {
    Number(u16),
    Register(usize),
    Invalid,
}
impl Value {
    fn new(v: u16) -> Self {
        match v {
            0..=32767 => Value::Number(v),
            32768..=32775 => Value::Register((v - 32768) as usize),
            32776..=65535 => Value::Invalid,
        }
    }
}
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(arg0) => write!(f, "{}", arg0),
            Self::Register(arg0) => f.debug_tuple("Reg").field(arg0).finish(),
            Self::Invalid => write!(f, "Invalid"),
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Opcode {
    Halt = 1 << 0,
    Set(Value, Value) = 1 << 1,
    Push(Value) = 1 << 2,
    Pop(Value) = 1 << 3,
    Eq(Value, Value, Value) = 1 << 4,
    Gt(Value, Value, Value) = 1 << 5,
    Jmp(Value) = 1 << 6,
    Jt(Value, Value) = 1 << 7,
    Jf(Value, Value) = 1 << 8,
    Add(Value, Value, Value) = 1 << 9,
    Mult(Value, Value, Value) = 1 << 10,
    Mod(Value, Value, Value) = 1 << 11,
    And(Value, Value, Value) = 1 << 12,
    Or(Value, Value, Value) = 1 << 13,
    Not(Value, Value) = 1 << 14,
    Rmem(Value, Value) = 1 << 15,
    Wmem(Value, Value) = 1 << 16,
    Call(Value) = 1 << 17,
    Ret = 1 << 18,
    Out(Value) = 1 << 19,
    In(Value) = 1 << 20,
    Noop = 1 << 21,
}

impl Opcode {
    pub fn discriminant(&self) -> u32 {
        unsafe { *(self as *const Self as *const u32) }
    }
}

const MEM_SIZE: usize = 32768;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Vm {
    //#[serde_as(as = "[_; MEM_SIZE]")]
    memory: Vec<u16>,
    registers: [u16; 8],
    stack: Vec<u16>,
    /// Instruction Pointer (next instruction)
    ip: usize,
    /// Program Counter
    pc: usize,

    state: VmState,

    output_buffer: Vec<char>,
    input_buffer: VecDeque<char>,

    messages: Vec<String>,

    traced_opcodes: u32,
    trace_buffer: Vec<(usize, Opcode)>,
}

impl PartialEq for Vm {
    fn eq(&self, other: &Self) -> bool {
        for (x, y) in self.memory.iter().zip(other.memory.iter()) {
            if x != y {
                return false;
            }
        }
        if self.registers != other.registers {
            return false;
        }
        if self.ip != other.ip {
            return false;
        }
        if self.pc != other.pc {
            return false;
        }
        if self.output_buffer != other.output_buffer {
            return false;
        }
        if self.input_buffer != other.input_buffer {
            return false;
        }

        true
    }
}

impl fmt::Debug for Vm {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "VM {{")?;
        writeln!(f, "  registers: {:?}", self.registers)?;
        writeln!(f, "  stack: {:?}", self.stack)?;
        writeln!(f, "  ip: {:?}", self.ip)?;
        writeln!(f, "  pc: {:?}", self.pc)?;
        writeln!(f, "  state: {:?}", self.state)?;
        writeln!(f, "  memory: [...]")?;
        write!(f, "}}")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VmState {
    Running,
    Halted,
    WaitingForInput,
}

impl Vm {
    fn new() -> Self {
        Vm {
            memory: vec![0u16; MEM_SIZE],
            registers: [0u16; 8],
            stack: Vec::new(),
            ip: 0,
            pc: 0,

            state: VmState::Running,

            output_buffer: Vec::new(),
            input_buffer: VecDeque::new(),

            messages: Vec::new(),

            traced_opcodes: 0,
            trace_buffer: Vec::new(),
        }
    }

    pub fn default() -> Self {
        let mut vm = Vm::new();
        vm.load_program_from_file("challenge.bin")
            .expect("Unable to load default 'challenge.bin'");

        vm
    }

    pub fn load_program_from_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut f = File::open(path)?;
        let mut buff = Vec::new();

        f.read_to_end(&mut buff)?;

        let data: Vec<_> = buff.chunks(2).map(LittleEndian::read_u16).collect();

        if data.len() > MEM_SIZE {
            panic!("File is too big");
        }
        self.memory[..data.len()].copy_from_slice(&data[..]);

        Ok(())
    }

    pub fn load_program_from_mem(&mut self, program: &[u16]) {
        self.memory[..program.len()].copy_from_slice(program);
    }

    pub fn get_messages(&self) -> &[String] {
        &self.messages
    }

    pub fn get_state(&self) -> VmState {
        self.state
    }

    pub fn set_register(&mut self, reg: usize, value: u16) {
        self.registers[reg] = value;
    }

    pub fn set_traced_opcodes(&mut self, traced: u32) {
        self.traced_opcodes = traced;
    }

    pub fn get_trace_buffer(&self) -> &[(usize, Opcode)] {
        &self.trace_buffer
    }

    pub fn disassemble(&self, mut from: usize, mut count: usize) -> Vec<(usize, Opcode)> {
        let mut instructions = Vec::new();

        while count > 0 {
            let (instr, size) = self.fetch(from);
            instructions.push((from, instr));

            from += size;
            count -= 1;
        }

        instructions
    }

    pub fn run(&mut self) {
        self.state = VmState::Running;

        while self.state == VmState::Running {
            self.step().unwrap();
        }

        if self.state == VmState::Halted {
            let message = self.output_buffer.iter().collect::<String>();
            self.messages.push(message.clone());
            println!("\n\nHalted");
        }
    }

    pub fn feed(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != VmState::WaitingForInput {
            return Err(format!("State is {:?}, can't feed", self.state).into());
        }
        if !self.input_buffer.is_empty() {
            return Err("Trying to feed but buffer is not empty".into());
        }

        self.input_buffer = line.chars().collect();
        self.input_buffer.push_back('\n');
        self.state = VmState::Running;

        Ok(())
    }

    pub fn step(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != VmState::Running {
            return Err("Vm is not running".into());
        }

        let (instruction, size) = self.fetch(self.ip);

        if (instruction.discriminant() & self.traced_opcodes) != 0 {
            self.trace_buffer.push((self.ip, instruction));
        }

        let next_instruction_ptr = self.ip + size;
        self.execute(&instruction, next_instruction_ptr);
        self.pc += 1;

        Ok(())
    }

    fn fetch(&self, ip: usize) -> (Opcode, usize) {
        let instr_type = self.memory[ip];

        match instr_type {
            0 => (Opcode::Halt, 1),
            1 => (
                Opcode::Set(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            2 => (Opcode::Push(Value::new(self.memory[ip + 1])), 2),
            3 => (Opcode::Pop(Value::new(self.memory[ip + 1])), 2),
            4 => (
                Opcode::Eq(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            5 => (
                Opcode::Gt(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            6 => (Opcode::Jmp(Value::new(self.memory[ip + 1])), 3),
            7 => (
                Opcode::Jt(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            8 => (
                Opcode::Jf(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            9 => (
                Opcode::Add(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            10 => (
                Opcode::Mult(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            11 => (
                Opcode::Mod(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            12 => (
                Opcode::And(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            13 => (
                Opcode::Or(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                    Value::new(self.memory[ip + 3]),
                ),
                4,
            ),
            14 => (
                Opcode::Not(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            15 => (
                Opcode::Rmem(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            16 => (
                Opcode::Wmem(
                    Value::new(self.memory[ip + 1]),
                    Value::new(self.memory[ip + 2]),
                ),
                3,
            ),
            17 => (Opcode::Call(Value::new(self.memory[ip + 1])), 2),
            18 => (Opcode::Ret, 1),
            19 => (Opcode::Out(Value::new(self.memory[ip + 1])), 2),
            20 => (Opcode::In(Value::new(self.memory[ip + 1])), 2),
            21 => (Opcode::Noop, 1),
            x => unreachable!("Fetch: unknown instr '{}'", x),
        }
    }

    fn execute(&mut self, instruction: &Opcode, next_instruction_ptr: usize) {
        //println!("{:?}", instruction);

        self.ip = next_instruction_ptr;

        match instruction {
            Opcode::Halt => self.state = VmState::Halted,
            Opcode::Set(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Opcode::Push(a) => {
                let val = self.get_value(a).expect("Invalid number");

                self.stack.push(val);
            }
            Opcode::Pop(a) => {
                let val = self.stack.pop().expect("Pop: empty stack");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Opcode::Eq(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b == val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Opcode::Gt(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b > val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Opcode::Jmp(a) => {
                self.ip = self.get_value(a).expect("Invalid number") as usize;
            }
            Opcode::Jt(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") != 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Opcode::Jf(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") == 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Opcode::Add(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b + val_c) % 32768; //TODO wrapping_add?
            }
            Opcode::Mult(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b.wrapping_mul(val_c) % 32768;
            }
            Opcode::Mod(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b % val_c;
            }
            Opcode::And(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b & val_c) % 32768;
            }
            Opcode::Or(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b | val_c) % 32768;
            }
            Opcode::Not(a, b) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (!val_b) % 32768;
            }
            Opcode::Rmem(a, b) => {
                let addr = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                let val = self.memory[addr as usize];

                self.registers[reg] = val;
            }
            Opcode::Wmem(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let addr = self.get_value(a).expect("Not a register");

                self.memory[addr as usize] = val;
            }
            Opcode::Call(a) => {
                let addr = self.get_value(a).expect("Invalid number");

                self.stack.push(self.ip as u16);
                self.ip = addr as usize;
            }
            Opcode::Ret => match self.stack.pop() {
                Some(addr) => self.ip = addr as usize,
                None => self.state = VmState::Halted,
            },
            Opcode::Out(a) => {
                let c = self.get_value(a).expect("Invalid number");

                self.output_buffer.push(c as u8 as char);
            }
            Opcode::In(a) => {
                let reg = self.get_register(a).expect("In: not a register");

                match self.input_buffer.pop_front() {
                    Some(c) => {
                        // just feed the current input
                        self.registers[reg] = c as u16;
                    }
                    None => {
                        // asking for new input
                        // first, flush current output
                        let out = self.output_buffer.iter().collect::<String>(); //TODO: separate function
                        self.messages.push(out.clone());
                        self.output_buffer = Vec::new();

                        self.state = VmState::WaitingForInput;
                        self.ip -= 2; // size of `In` instruction
                    }
                }
            }
            Opcode::Noop => (),
        }
    }

    fn get_value(&self, value: &Value) -> Option<u16> {
        match value {
            Value::Number(x) => Some(*x),
            Value::Register(x) => Some(self.registers[*x]),
            Value::Invalid => None,
        }
    }

    fn get_register(&self, value: &Value) -> Option<usize> {
        match value {
            Value::Number(_) => None,
            Value::Register(x) => Some(*x),
            Value::Invalid => None,
        }
    }
}
