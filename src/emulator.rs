use std::{
    collections::{HashMap, VecDeque},
    fmt,
    fs::File,
    io::Read,
    path::Path,
};

use std::fmt::Debug;

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::assembly::{Opcode, Val};

const MEM_SIZE: usize = 32768;

#[serde_as]
#[derive(Clone, Serialize, Deserialize)]
pub struct Vm {
    memory: Vec<u16>,
    registers: [u16; 8],
    stack: Vec<u16>,
    /// Instruction Pointer (next instruction)
    ip: usize,
    /// Program Counter
    pc: usize,

    state: VmState,

    /// Current output buffer
    output_buffer: Vec<char>,
    input_buffer: VecDeque<char>,
    /// Old output buffers
    messages: Vec<String>,

    traced_opcodes: u32,
    #[serde(skip)]
    trace_buffer: Vec<(usize, Opcode, Option<Opcode>)>,

    #[serde(skip)]
    called_patched_fn: bool,
    #[serde(skip)]
    fn_patching: bool,

    #[serde(skip)]
    breakpoints: Vec<usize>,

    #[serde(skip)]
    __6027_cache: HashMap<(u16, u16, u16), (u16, u16)>,

    #[serde(skip)]
    scanmem: Vec<Option<u16>>,
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
        let output_buffer: String = self.output_buffer.iter().collect();

        writeln!(f, "VM {{")?;
        writeln!(f, "  registers: {:?}", self.registers)?;
        writeln!(f, "  stack: {:?}", self.stack)?;
        writeln!(f, "  ip: {:?}", self.ip)?;
        writeln!(f, "  pc: {:?}", self.pc)?;
        writeln!(f, "  state: {:?}", self.state)?;
        writeln!(f, "  patching: {:?}", self.fn_patching)?;
        writeln!(f, "  output_buffer: {:?}", output_buffer)?;
        writeln!(f, "  memory: [...]")?;
        write!(f, "}}")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VmState {
    Running,
    Halted,
    WaitingForInput,
    HitBreakPoint,
}
impl Default for Vm {
    fn default() -> Self {
        let mut vm = Vm::new();
        vm.load_program_from_file("challenge.bin")
            .expect("Unable to load default 'challenge.bin'");

        vm
    }
}

pub trait StopCondition {
    fn must_stop(&mut self, vm: &Vm) -> Result<bool, Box<dyn std::error::Error>>;
}

pub struct StopNever;

impl StopCondition for StopNever {
    fn must_stop(&mut self, _vm: &Vm) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

pub struct StopVmState {
    states: Vec<VmState>,
}

impl StopVmState {
    pub fn new(states: &[VmState]) -> Self {
        Self {
            states: states.to_vec(),
        }
    }
}

impl StopCondition for StopVmState {
    fn must_stop(&mut self, vm: &Vm) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.states.contains(&vm.state))
    }
}

#[derive(Default)]
pub struct StopRet {
    ret_counter: i32,
}

impl StopCondition for StopRet {
    fn must_stop(&mut self, vm: &Vm) -> Result<bool, Box<dyn std::error::Error>> {
        let instr = vm.fetch(vm.ip).unwrap();
        match instr {
            Opcode::Ret => {
                self.ret_counter -= 1;

                if self.ret_counter < 0 {
                    return Err("ret_counter < 0".into());
                }

                if self.ret_counter == 0 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Opcode::Call(_) => {
                self.ret_counter += 1;
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

pub struct StopInstructionCounter {
    instuction_counter: usize,
}

impl StopCondition for StopInstructionCounter {
    fn must_stop(&mut self, _vm: &Vm) -> Result<bool, Box<dyn std::error::Error>> {
        if self.instuction_counter == 0 {
            Ok(true)
        } else {
            self.instuction_counter -= 1;
            Ok(false)
        }
    }
}

impl<T: StopCondition + 'static> From<T> for Box<dyn StopCondition> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl Vm {
    pub fn run_until<S>(
        &mut self,
        stop_condition: S,
    ) -> Result<Vec<(usize, Opcode)>, Box<dyn std::error::Error>>
    where
        S: Into<Box<dyn StopCondition>>,
    {
        let mut stop_condition = stop_condition.into();
        let mut executed = Vec::new();

        while !stop_condition.must_stop(self)? {
            let instr = self.step().unwrap();
            executed.push(instr);
        }

        if self.state == VmState::Halted {
            let message = self.output_buffer.iter().collect::<String>();
            self.messages.push(message.clone());
        }
        if self.state == VmState::HitBreakPoint {
            println!("Hit breakpoint at {}", self.ip);
        }

        // loop {
        //     let opcode = if self.called_patched_fn {
        //         self.called_patched_fn = false;
        //         Opcode::Ret
        //     } else {
        //         self.fetch(self.ip)?
        //     };
        //     let must_stop = stop_condition.must_stop(&self);

        //     let opcode = self.fetch(self.ip)?;
        //     let next_instruction_ptr = self.ip + opcode.size();
        //     executed.push((self.ip, opcode));
        //     self.execute(&opcode, next_instruction_ptr);

        //     if must_stop {
        //         break;
        //     }
        // }

        Ok(executed)
    }

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

            fn_patching: false,
            called_patched_fn: false,

            breakpoints: Vec::new(),

            __6027_cache: HashMap::new(),

            scanmem: vec![None; MEM_SIZE],
        }
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

    pub fn get_registers(&self) -> &[u16; 8] {
        &self.registers
    }

    pub fn set_traced_opcodes(&mut self, traced: u32) {
        self.traced_opcodes = traced;
    }

    pub fn get_trace_buffer(&self) -> &[(usize, Opcode, Option<Opcode>)] {
        &self.trace_buffer
    }

    pub fn set_fn_patching(&mut self, val: bool) {
        self.fn_patching = val;
    }

    pub fn get_breakpoints(&self) -> &[usize] {
        &self.breakpoints
    }

    pub fn set_breakpoint(&mut self, offset: usize) {
        if !self.breakpoints.contains(&offset) {
            self.breakpoints.push(offset);
        }
    }

    pub fn unset_breakpoint(&mut self, offset: usize) {
        self.breakpoints.retain(|bp| *bp != offset);
    }

    pub fn scanmem_init(&mut self) {
        self.scanmem = vec![None; MEM_SIZE];
        for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
            *b = Some(*a);
        }
    }

    pub fn mem_set(&mut self, offset: usize, value: u16) {
        self.memory[offset] = value;
    }

    pub fn mem_get(&mut self, offset: usize) {
        println!("{}: {}", offset, self.memory[offset]);
    }

    pub fn scanmem_list(&self) {
        for (idx, (mem, scanmem)) in self.memory.iter().zip(self.scanmem.iter()).enumerate() {
            if let Some(scanmem) = scanmem {
                println!("{}: {} -> {}", idx, scanmem, mem);
            }
        }

        let count = self.scanmem.iter().filter(|x| x.is_some()).count();
        println!("Listed {} values", count);
    }

    pub fn scanmem_filter(&mut self, op: &str, val: Option<u16>) {
        match op {
            "=" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b == cmp => continue,
                        Some(b) if *b != cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }
            "!=" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b != cmp => continue,
                        Some(b) if *b == cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }
            ">" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b > cmp => continue,
                        Some(b) if *b <= cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }
            ">=" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b >= cmp => continue,
                        Some(b) if *b < cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }
            "<" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b < cmp => continue,
                        Some(b) if *b >= cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }
            "<=" => {
                for (a, b) in self.memory.iter().zip(self.scanmem.iter_mut()) {
                    let cmp = if let Some(val) = val { val } else { *a };
                    match b {
                        Some(b) if *b <= cmp => continue,
                        Some(b) if *b > cmp => (),
                        Some(_) => unreachable!(),
                        None => (),
                    }
                    *b = None;
                }
            }

            x => println!("unknown op {:?}", x),
        }

        let count = self.scanmem.iter().filter(|x| x.is_some()).count();
        println!("Selected {} values", count);
    }

    pub fn patch(&mut self, opcode: Opcode, offset: usize) {
        let bin = opcode.assemble();
        let size = bin.len();

        match self.disassemble(offset, 1) {
            Ok(x) => {
                let old_size = x.len();
                if old_size != size {
                    println!("WARNING: patched opcode of different size");
                }
            }
            Err(e) => println!("Can't disassemble {:?}", e),
        }

        self.memory[offset..(offset + size)].copy_from_slice(&bin);
    }

    /// $ dis fn 2125
    /// 2125: Push(Reg(1))
    /// 2127: Push(Reg(2))
    /// 2129: And(Reg(2), Reg(0), Reg(1))
    /// 2133: Not(Reg(2), Reg(2))        
    /// 2136: Or(Reg(0), Reg(0), Reg(1))
    /// 2140: And(Reg(0), Reg(0), Reg(2))
    /// 2144: Pop(Reg(2))
    /// 2146: Pop(Reg(1))
    /// 2148: Ret
    #[allow(dead_code)]
    fn patched_2125(&mut self) {
        fn op(mut reg0: u16, reg1: u16) -> u16 {
            let mut reg2 = reg0 & reg1;
            reg2 = !reg2;
            reg0 |= reg1;
            reg0 &= reg2;

            reg0
        }

        let reg0 = self.registers[0];
        let reg1 = self.registers[1];

        self.registers[0] = op(reg0, reg1);
        self.pc += 9;
    }

    /// $ dis fn 6027
    /// 6027: Jt(Reg(0), 6035)
    /// 6030: Add(Reg(0), Reg(1), 1)
    /// 6034: Ret
    /// 6035: Jt(Reg(1), 6048)
    /// 6038: Add(Reg(0), Reg(0), 32767)
    /// 6042: Set(Reg(1), Reg(7))
    /// 6045: Call(6027)
    /// 6047: Ret
    /// 6048: Push(Reg(0))
    /// 6050: Add(Reg(1), Reg(1), 32767)
    /// 6054: Call(6027)
    /// 6056: Set(Reg(1), Reg(0))
    /// 6059: Pop(Reg(0))
    /// 6061: Add(Reg(0), Reg(0), 32767)
    /// 6065: Call(6027)
    /// 6067: Ret
    ///
    /// [src\emulator.rs:727] self = VM {
    /// registers: [4, 1, 3, 10, 101, 0, 0, 1]
    /// stack: [6080, 16, 6124, 1, 2952, 25978, 3568, 3599, 2708, 5445, 3]
    /// ip: 5491
    /// pc: 1012532
    /// state: Running
    /// memory: [...]
    /// }
    ///
    /// Called at
    /// 5489: Call(6027)
    /// 5491: Eq(Reg(1), Reg(0), 6)
    /// 5495: Jf(Reg(1), 5579)
    /// 5498: Push(Reg(0))
    /// 5500: Push(Reg(1))
    /// 5502: Push(Reg(2))
    /// 5504: Set(Reg(0), 29014)
    /// 5507: Set(Reg(1), 1531)
    /// 5510: Add(Reg(2), 21718, 1807)
    /// 5514: Call(1458)
    #[allow(unused_assignments)]
    fn patched_6027(&mut self, mut r0: u16, mut r1: u16, r7: u16) -> (u16, u16) {
        let init_r0 = r0;
        let init_r1 = r1;
        let init_r7 = r7;
        if let Some(x) = self.__6027_cache.get(&(r0, r1, r7)) {
            return *x;
        }

        if r0 != 0 {
            if r1 != 0 {
                let old_r0 = r0;
                r1 -= 1;
                (r0, r1) = self.patched_6027(r0, r1, r7);
                r1 = r0;
                r0 = old_r0;
                r0 -= 1;
                let (r0, r1) = self.patched_6027(r0, r1, r7);
                self.__6027_cache
                    .insert((init_r0, init_r1, init_r7), (r0, r1));
                (r0, r1)
            } else {
                r0 -= 1;
                r1 = r7;
                let (r0, r1) = self.patched_6027(r0, r1, r7);
                self.__6027_cache
                    .insert((init_r0, init_r1, init_r7), (r0, r1));
                (r0, r1)
            }
        } else {
            r0 = r1 + 1;
            (r0, r1)
        }
    }

    pub fn disassemble(
        &self,
        mut start: usize,
        mut count: usize,
    ) -> Result<Vec<(usize, Opcode)>, Box<dyn std::error::Error>> {
        let mut instructions = Vec::new();

        while count > 0 {
            let instr = self.fetch(start)?;
            let size = instr.size();
            instructions.push((start, instr));

            start += size;
            count -= 1;
        }

        Ok(instructions)
    }

    /// Disassemble from starting `Call` of function to all `Ret`
    /// don't expect self modifying code
    pub fn disassemble_function(
        &self,
        starting_ip: usize,
    ) -> Result<Function, Box<dyn std::error::Error>> {
        //) -> Result<Vec<(usize, Opcode)>, Box<dyn std::error::Error>> {
        let mut instructions = Vec::new();

        let mut explored: Vec<usize> = Vec::new();
        let mut queue = VecDeque::new();
        let instr = self.fetch(starting_ip)?;
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

                let opcode = self.fetch(ip)?;
                let size = opcode.size();
                queue.push_back((ip, opcode, size));
            }

            explored.push(ip);
            instructions.push((ip, instr));
        }

        instructions.sort_by_key(|a| a.0);

        let first_offset = instructions.first().unwrap().0;
        let last_offset = instructions.last().unwrap().0;

        let instructions: Vec<Opcode> = instructions.into_iter().map(|(_, op)| op).collect();

        let function = Function::new(first_offset, last_offset, &instructions);
        Ok(function)
    }

    pub fn pretty_print_dis(instructions: &[(usize, Opcode)]) {
        // TODO: remove
        let mut last: Option<(usize, Opcode)> = None;
        for &(offset, opcode) in instructions.iter() {
            if let Some((previous_offset, previous_opcode)) = last {
                if previous_opcode.size() + previous_offset < offset {
                    println!("[...]");
                }
            }

            println!("{}: {:?}", offset, opcode);
            last = Some((offset, opcode));
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

    pub fn step(&mut self) -> Result<(usize, Opcode), Box<dyn std::error::Error>> {
        if self.state != VmState::Running {
            return Err(format!("Vm is not running: {:?}.", self.state).into());
        }

        if self.breakpoints.contains(&self.ip) {
            self.state = VmState::HitBreakPoint;
            todo!();
            //return Ok(());
        }

        let ip = self.ip;
        let opcode = self.fetch(self.ip)?;
        let size = opcode.size();

        if (opcode.discriminant() & self.traced_opcodes) != 0 {
            let resolved_opcode = opcode.resolve_opcode(self);
            self.trace_buffer.push((self.ip, opcode, resolved_opcode));
        }

        let next_instruction_ptr = self.ip + size;
        self.execute(&opcode, next_instruction_ptr)?;
        self.pc += 1;

        Ok((ip, opcode))
    }

    /// Return `Opcode)` decoded at `ip`
    fn fetch(&self, ip: usize) -> Result<Opcode, Box<dyn std::error::Error>> {
        Opcode::fetch(&self.memory, ip)
    }

    fn execute(
        &mut self,
        instruction: &Opcode,
        next_instruction_ptr: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use Opcode::*;

        //println!("{:?}", instruction);

        self.ip = next_instruction_ptr;

        match instruction {
            Halt => self.state = VmState::Halted,
            Set(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Push(a) => {
                let val = self.get_value(a).expect("Invalid number");

                self.stack.push(val);
            }
            Pop(a) => {
                let val = self.stack.pop().expect("Pop: empty stack");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Eq(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b == val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Gt(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b > val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Jmp(a) => {
                self.ip = self.get_value(a).expect("Invalid number") as usize;
            }
            Jt(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") != 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Jf(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") == 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Add(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b + val_c) % 32768; //TODO wrapping_add?
            }
            Mult(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b.wrapping_mul(val_c) % 32768;
            }
            Mod(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b % val_c;
            }
            And(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b & val_c) % 32768;
            }
            Or(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b | val_c) % 32768;
            }
            Not(a, b) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (!val_b) % 32768;
            }
            Rmem(a, b) => {
                let addr = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                let val = self.memory[addr as usize];

                self.registers[reg] = val;
            }
            Wmem(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let addr = self.get_value(a).expect("Not a register");

                self.memory[addr as usize] = val;
            }
            Call(a) => {
                let addr = self.get_value(a).expect("Invalid number");

                //dbg!(addr);
                if self.fn_patching {
                    match addr {
                        3 => {
                            self.stack.push(self.ip as u16);
                            {
                                // function code
                                self.registers[0] = 20;
                            }
                            self.called_patched_fn = true;
                            return Ok(());
                        }
                        2125 => {
                            let mut test_vm = self.clone();
                            test_vm.run_until(StopRet::default())?;

                            self.stack.push(self.ip as u16);
                            self.patched_2125();
                            self.called_patched_fn = true;

                            assert_eq!(&test_vm, self);
                            return Ok(());
                        }
                        6027 => {
                            self.stack.push(self.ip as u16);
                            let (r0, r1) = self.patched_6027(
                                self.registers[0],
                                self.registers[1],
                                self.registers[7],
                            );
                            self.registers[0] = r0;
                            self.registers[1] = r1;
                            self.called_patched_fn = true;
                            return Ok(());
                        }
                        _ => (),
                    }
                }

                self.stack.push(self.ip as u16);
                self.ip = addr as usize;
            }
            Ret => match self.stack.pop() {
                Some(addr) => {
                    self.ip = addr as usize;
                }
                None => {
                    self.state = {
                        println!("poped empty stack!");
                        VmState::Halted
                    }
                }
            },
            Out(a) => {
                let c = self.get_value(a).expect("Invalid number");

                self.output_buffer.push(c as u8 as char);
            }
            In(a) => {
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
            Noop => (),
            __Invalid => panic!("Got __Invalid Opcode at {}", self.ip),
        }

        Ok(())
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

    pub fn get_mem(&self) -> &[u16] {
        &self.memory
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Function {
    pub start: usize,
    pub end: usize,
    code: Vec<Opcode>,
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (offset, opcode) in self.code.iter().enumerate() {
            write!(f, "{} {:?}", offset + self.start, opcode)?;
        }

        Ok(())
    }
}

impl Function {
    pub fn new(start: usize, end: usize, code: &[Opcode]) -> Self {
        let offset_size = end - start;
        let machine_code = Opcode::assemble_vec(code);
        let machine_code_size = machine_code.len();

        // instructions must be a continuous chuck of memory (no gaps)
        assert_eq!(offset_size + code.last().unwrap().size(), machine_code_size);

        Self {
            start,
            end,
            code: code.to_vec(),
        }
    }

    pub fn get_code(&self) -> &[Opcode] {
        &self.code
    }

    pub fn pretty_print(&self) {
        let mut ptr = self.start;

        for op in &self.code {
            println!("{} {:?}", ptr, op);
            ptr += op.size();
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    pub fn merge_biggest<'a>(&'a self, other: &'a Self) -> &'a Self {
        if self.contains(other) {
            self
        } else {
            other
        }
    }

    pub fn graphviz(&self) -> String {
        let name = format!("{}", self.start);

        let mut s = format!("\"{name}\" [\n");
        s.push_str("shape=\"none\"\n");
        s.push_str("label=<\n");
        s.push_str("<table>\n");
        s.push_str(&format!(
            "    <tr><td bgcolor=\"black\" colspan=\"2\" port=\"0\"><font color=\"white\">{}</font></td></tr>\n",
            self.start
        ));
        for (offset, op) in self.code.iter().enumerate() {
            let offset = offset + self.start;
            s.push_str(&format!(
                "    <tr><td align=\"left\" port=\"{offset}\">{offset}</td><td>{op :?}</td></tr>\n",
            ));
        }
        s.push_str("</table>\n");
        s.push_str(">\n");
        s.push_str("];\n\n");

        s
    }
}
