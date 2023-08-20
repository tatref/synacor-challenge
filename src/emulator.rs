use std::{
    collections::{BTreeSet, VecDeque},
    fmt,
    fs::File,
    io::Read,
    path::Path,
};

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Val {
    Num(u16),
    Reg(usize),
    Invalid,
}
impl Val {
    fn new(v: u16) -> Self {
        match v {
            0..=32767 => Val::Num(v),
            32768..=32775 => Val::Reg((v - 32768) as usize),
            32776..=65535 => Val::Invalid,
        }
    }

    fn to_binary(&self) -> u16 {
        match self {
            Val::Num(v) => *v,
            Val::Reg(r) => *r as u16 + 32768,
            Val::Invalid => 32776,
        }
    }
}
impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num(arg0) => write!(f, "{}", arg0),
            Self::Reg(arg0) => f.debug_tuple("Reg").field(arg0).finish(),
            Self::Invalid => write!(f, "Invalid"),
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Opcode {
    Halt = 1 << 0,
    Set(Val, Val) = 1 << 1,
    Push(Val) = 1 << 2,
    Pop(Val) = 1 << 3,
    Eq(Val, Val, Val) = 1 << 4,
    Gt(Val, Val, Val) = 1 << 5,
    /// jump to `a`
    Jmp(Val) = 1 << 6,
    /// if `a` is nonzero, jump to `b`
    Jt(Val, Val) = 1 << 7,
    /// if `a`  is zero, jump to `b`
    Jf(Val, Val) = 1 << 8,
    Add(Val, Val, Val) = 1 << 9,
    Mult(Val, Val, Val) = 1 << 10,
    Mod(Val, Val, Val) = 1 << 11,
    And(Val, Val, Val) = 1 << 12,
    Or(Val, Val, Val) = 1 << 13,
    Not(Val, Val) = 1 << 14,
    Rmem(Val, Val) = 1 << 15,
    Wmem(Val, Val) = 1 << 16,
    /// write the address of the next instruction to the stack and jump to `a`
    Call(Val) = 1 << 17,
    /// remove the top element from the stack and jump to it; empty stack = halt
    Ret = 1 << 18,
    Out(Val) = 1 << 19,
    In(Val) = 1 << 20,
    Noop = 1 << 21,
}

impl Opcode {
    pub fn discriminant(&self) -> u32 {
        unsafe { *(self as *const Self as *const u32) }
    }

    pub fn size(&self) -> usize {
        match self {
            Opcode::Halt => 1,
            Opcode::Set(_, _) => 3,
            Opcode::Push(_) => 2,
            Opcode::Pop(_) => 2,
            Opcode::Eq(_, _, _) => 4,
            Opcode::Gt(_, _, _) => 4,
            Opcode::Jmp(_) => 2,
            Opcode::Jt(_, _) => 3,
            Opcode::Jf(_, _) => 3,
            Opcode::Add(_, _, _) => 4,
            Opcode::Mult(_, _, _) => 4,
            Opcode::Mod(_, _, _) => 4,
            Opcode::And(_, _, _) => 4,
            Opcode::Or(_, _, _) => 4,
            Opcode::Not(_, _) => 3,
            Opcode::Rmem(_, _) => 3,
            Opcode::Wmem(_, _) => 3,
            Opcode::Call(_) => 2,
            Opcode::Ret => 1,
            Opcode::Out(_) => 2,
            Opcode::In(_) => 2,
            Opcode::Noop => 1,
        }
    }

    /// Next pointer for branchings instructions
    pub fn next_possible_ip(&self) -> Vec<Val> {
        match self {
            Opcode::Halt => vec![],
            Opcode::Set(_, _) => vec![],
            Opcode::Push(_) => vec![],
            Opcode::Pop(_) => vec![],
            Opcode::Eq(_, _, _) => vec![],
            Opcode::Gt(_, _, _) => vec![],
            Opcode::Jmp(a) => vec![*a],
            Opcode::Jt(_, b) => vec![*b],
            Opcode::Jf(_, b) => vec![*b],
            Opcode::Add(_, _, _) => vec![],
            Opcode::Mult(_, _, _) => vec![],
            Opcode::Mod(_, _, _) => vec![],
            Opcode::And(_, _, _) => vec![],
            Opcode::Or(_, _, _) => vec![],
            Opcode::Not(_, _) => vec![],
            Opcode::Rmem(_, _) => vec![],
            Opcode::Wmem(_, _) => vec![],
            Opcode::Call(a) => vec![*a],
            Opcode::Ret => vec![],
            Opcode::Out(_) => vec![],
            Opcode::In(_) => vec![],
            Opcode::Noop => vec![],
        }
    }

    pub fn machine_code(&self) -> Vec<u16> {
        match self {
            Opcode::Halt => vec![0],
            Opcode::Set(a, b) => vec![1, a.to_binary(), b.to_binary()],
            Opcode::Push(_) => todo!(),
            Opcode::Pop(_) => todo!(),
            Opcode::Eq(a, b, c) => vec![4, a.to_binary(), b.to_binary(), c.to_binary()],
            Opcode::Gt(_, _, _) => todo!(),
            Opcode::Jmp(a) => vec![6, a.to_binary()],
            Opcode::Jt(a, b) => vec![7, a.to_binary(), b.to_binary()],
            Opcode::Jf(a, b) => vec![8, a.to_binary(), b.to_binary()],
            Opcode::Add(a, b, c) => vec![9, a.to_binary(), b.to_binary(), c.to_binary()],
            Opcode::Mult(_, _, _) => todo!(),
            Opcode::Mod(_, _, _) => todo!(),
            Opcode::And(_, _, _) => todo!(),
            Opcode::Or(_, _, _) => todo!(),
            Opcode::Not(_, _) => todo!(),
            Opcode::Rmem(_, _) => todo!(),
            Opcode::Wmem(_, _) => todo!(),
            Opcode::Call(a) => vec![17, a.to_binary()],
            Opcode::Ret => vec![18],
            Opcode::Out(_) => todo!(),
            Opcode::In(_) => todo!(),
            Opcode::Noop => vec![21],
        }
    }

    pub fn vec_to_machine_code(v: &[Opcode]) -> Vec<u16> {
        let mut machine_code = Vec::new();

        for opcode in v {
            machine_code.extend(&opcode.machine_code());
        }

        machine_code
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
    pub fn new() -> Self {
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

    pub fn disassemble(&self, mut start: usize, mut count: usize) -> Vec<(usize, Opcode)> {
        let mut instructions = Vec::new();

        while count > 0 {
            let instr = self.fetch(start);
            let size = instr.size();
            instructions.push((start, instr));

            start += size;
            count -= 1;
        }

        instructions
    }

    /// Disassemble from starting `Call` of function to all `Ret`
    /// we don't expecte self modifying code
    pub fn disassemble_function(&self, starting_ip: usize) -> Vec<(usize, Opcode)> {
        let mut instructions = Vec::new();

        let mut explored: Vec<usize> = Vec::new();
        let mut queue = VecDeque::new();
        let instr = self.fetch(starting_ip);
        let size = instr.size();
        queue.push_back((starting_ip, instr, size));

        while let Some((ip, instr, size)) = queue.pop_front() {
            if explored.contains(&ip) {
                continue;
            }

            //let mut next = instr.next_possible_ip(); // possible branches
            let next: Vec<Val> = match instr {
                Opcode::Halt => vec![],
                Opcode::Ret => vec![],
                Opcode::Call(_) => vec![Val::Num(ip as u16 + size as u16)], // don't follow calls
                _ => {
                    let mut next = instr.next_possible_ip();
                    next.push(Val::Num(ip as u16 + size as u16));
                    next
                }
            };

            for n in &next {
                let ip = match n {
                    Val::Invalid => continue,
                    Val::Reg(_r) => continue,
                    Val::Num(x) => *x as usize,
                };

                if explored.contains(&ip) {
                    continue;
                }

                let opcode = self.fetch(ip);
                let size = opcode.size();
                queue.push_back((ip, opcode, size));
            }

            explored.push(ip);
            instructions.push((ip, instr));
        }

        instructions.sort_by_key(|a| a.0);
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

        let instruction = self.fetch(self.ip);
        let size = instruction.size();

        if (instruction.discriminant() & self.traced_opcodes) != 0 {
            self.trace_buffer.push((self.ip, instruction));
        }

        let next_instruction_ptr = self.ip + size;
        self.execute(&instruction, next_instruction_ptr);
        self.pc += 1;

        Ok(())
    }

    /// Return `Opcode)` decoded at `ip`
    fn fetch(&self, ip: usize) -> Opcode {
        let instr_type = self.memory[ip];

        match instr_type {
            0 => Opcode::Halt,
            1 => Opcode::Set(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            2 => Opcode::Push(Val::new(self.memory[ip + 1])),
            3 => Opcode::Pop(Val::new(self.memory[ip + 1])),
            4 => Opcode::Eq(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            5 => Opcode::Gt(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            6 => Opcode::Jmp(Val::new(self.memory[ip + 1])),
            7 => Opcode::Jt(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            8 => Opcode::Jf(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            9 => Opcode::Add(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            10 => Opcode::Mult(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            11 => Opcode::Mod(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            12 => Opcode::And(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            13 => Opcode::Or(
                Val::new(self.memory[ip + 1]),
                Val::new(self.memory[ip + 2]),
                Val::new(self.memory[ip + 3]),
            ),
            14 => Opcode::Not(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            15 => Opcode::Rmem(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            16 => Opcode::Wmem(Val::new(self.memory[ip + 1]), Val::new(self.memory[ip + 2])),
            17 => Opcode::Call(Val::new(self.memory[ip + 1])),
            18 => Opcode::Ret,
            19 => Opcode::Out(Val::new(self.memory[ip + 1])),
            20 => Opcode::In(Val::new(self.memory[ip + 1])),
            21 => Opcode::Noop,
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

    fn get_value(&self, value: &Val) -> Option<u16> {
        match value {
            Val::Num(x) => Some(*x),
            Val::Reg(x) => Some(self.registers[*x]),
            Val::Invalid => None,
        }
    }

    fn get_register(&self, value: &Val) -> Option<usize> {
        match value {
            Val::Num(_) => None,
            Val::Reg(x) => Some(*x),
            Val::Invalid => None,
        }
    }
}
