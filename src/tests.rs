use itertools::iproduct;

use crate::assembly::{Opcode, Val};
use crate::emulator::{StopRet, Vm};

#[test]
fn load_program_from_file() -> Result<(), Box<dyn std::error::Error>> {
    use crate::emulator::Vm;

    let f = "challenge.bin";
    let mut vm = Vm::default();
    vm.load_program_from_file(f)
}

#[test]
fn load_program_from_mem() -> Result<(), Box<dyn std::error::Error>> {
    use crate::emulator::Vm;

    let mut vm = Vm::default();
    let program = [9, 32768, 32769, 4, 19, 32768];
    vm.load_program_from_mem(&program);

    Ok(())
}

#[test]
fn machine_code() -> Result<(), Box<dyn std::error::Error>> {
    let expected = vec![0];
    let prog = vec![Opcode::Halt];

    assert_eq!(Opcode::assemble_vec(&prog), expected);

    Ok(())
}

#[test]
fn disassemble_function() -> Result<(), Box<dyn std::error::Error>> {
    use Opcode::*;
    use Val::*;

    let prog = vec![
        Set(Reg(0), Num(2)),
        Eq(Reg(0), Reg(1), Num(2)),
        Jt(Num(0), Num(10)),
        Ret,
        Add(Reg(2), Num(1), Num(2)),
        Ret,
    ];
    let machine_code = Opcode::assemble_vec(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&machine_code);

    let function = vm.disassemble_function(0)?;
    function.pretty_print();

    Ok(())
}

#[test]
fn assemble_disassemble() {
    use Opcode::*;
    use Val::*;

    let prog = vec![
        Set(Reg(0), Num(2)),
        Eq(Reg(0), Reg(1), Num(2)),
        Jt(Num(0), Num(15)),
        Ret,
        Add(Reg(2), Num(1), Num(2)),
        Call(Num(2)),
        Call(Num(2)),
        Call(Num(2)),
        Call(Num(2)),
        Ret,
    ];
    let machine_code = Opcode::assemble_vec(&prog);

    let prog2: Vec<Opcode> = Opcode::disassemble(&machine_code, 0)
        .unwrap()
        .iter()
        .map(|(_, op)| op)
        .copied()
        .collect();

    assert_eq!(prog, prog2);
}

// 2125: Push(Reg(1))
// 2127: Push(Reg(2))
// 2129: And(Reg(2), Reg(0), Reg(1))
// 2133: Not(Reg(2), Reg(2))
// 2136: Or(Reg(0), Reg(0), Reg(1))
// 2140: And(Reg(0), Reg(0), Reg(2))
// 2144: Pop(Reg(2))
// 2146: Pop(Reg(1))
// 2148: Ret
#[test]
fn patching_2125() -> Result<(), Box<dyn std::error::Error>> {
    use Opcode::*;
    use Val::*;

    let mut prog = vec![Opcode::Call(Val::Num(2125))];
    prog.extend([Opcode::Noop].repeat(2125 - Opcode::Call(Val::Num(0)).size()));

    prog.extend_from_slice(&[
        Push(Reg(1)),
        Push(Reg(2)),
        And(Reg(2), Reg(0), Reg(1)),
        Not(Reg(2), Reg(2)),
        Or(Reg(0), Reg(0), Reg(1)),
        And(Reg(0), Reg(0), Reg(2)),
        Pop(Reg(2)),
        Pop(Reg(1)),
        Ret,
    ]);

    let prog = Opcode::assemble_vec(&prog);

    assert!(prog.len() == 2149);

    let mut vm = Vm::default();
    vm.load_program_from_mem(&prog);

    let mut vm1 = vm.clone();
    let mut vm2 = vm.clone();

    println!("vm1");
    let executed = vm1.run_until(StopRet::new()).unwrap();
    for (offset, op) in &executed {
        println!("{}: {:?}", offset, op);
    }

    println!();
    println!("vm2");
    vm2.set_fn_patching(true);
    let executed = vm2.run_until(StopRet::new()).unwrap();
    for (offset, op) in &executed {
        println!("{}: {:?}", offset, op);
    }

    assert_eq!(vm1, vm2);

    Ok(())
}

#[test]
fn run_until_ret_2125() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![Opcode::Call(Val::Num(2125))];
    let prog = Opcode::assemble_vec(&prog);

    for (reg0, reg1) in iproduct!(0..100, 0..100) {
        let mut vm = Vm::default();
        vm.load_program_from_mem(&prog);
        vm.set_register(0, reg0);
        vm.set_register(1, reg1);

        let mut vm1 = vm.clone();
        let mut vm2 = vm.clone();

        println!("vm1");
        let _instr = vm1.run_until(StopRet::new())?;

        println!("vm2");
        vm2.set_fn_patching(true);
        let _instr = vm2.run_until(StopRet::new())?;

        assert_eq!(vm1, vm2);
    }

    Ok(())
}

#[test]
#[allow(unreachable_code)]
fn patching_3() -> Result<(), Box<dyn std::error::Error>> {
    todo!();

    let prog = vec![
        Opcode::Call(Val::Num(3)),
        Opcode::Halt,
        Opcode::Set(Val::Reg(0), Val::Num(20)),
        Opcode::Ret,
    ];
    let prog = Opcode::assemble_vec(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&prog);

    let x = vm.disassemble(0, 5)?;
    Vm::pretty_print_dis(&x);

    let mut vm1 = vm.clone();
    let mut vm2 = vm.clone();

    println!("vm1");
    let executed = vm1.run_until(StopRet::new()).unwrap();
    for (offset, op) in &executed {
        println!("{}: {:?}", offset, op);
    }

    println!("vm2");
    vm2.set_fn_patching(true);
    let executed = vm1.run_until(StopRet::new()).unwrap();
    for (offset, op) in &executed {
        println!("xx {}: {:?}", offset, op);
    }

    panic!();

    assert_eq!(vm1, vm2);

    Ok(())
}

#[test]
fn run_until_ret() -> Result<(), Box<dyn std::error::Error>> {
    use Opcode::*;

    let prog = vec![
        Call(Val::Num(3)),
        Halt,
        Call(Val::Num(6)),
        Ret,
        Set(Val::Reg(0), Val::Num(20)),
        Ret,
        __Invalid,
    ];
    let prog = Opcode::assemble_vec(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&prog);

    let x = vm.disassemble(0, 7)?;
    println!("Disassembly:");
    Vm::pretty_print_dis(&x);
    println!();

    let executed = vm.run_until(StopRet::new()).unwrap();
    println!("Executed:");
    for (offset, op) in &executed {
        println!("xx {}: {:?}", offset, op);
    }

    // TODO: last Ret is not executed
    assert_eq!(executed.len(), 5);

    Ok(())
}

#[test]
fn parse_opcode() {
    let input = "Set(Reg(1), 1531)
Gt(Reg(1), Reg(2), Reg(1))
Jf(Reg(1), 5636)
Ret
Add(Reg(2), 10666, 956)";

    let instructions: Vec<Opcode> = input
        .lines()
        .map(|line| line.parse::<Opcode>().unwrap())
        .collect();

    let expected = [
        Opcode::Set(Val::Reg(1), Val::Num(1531)),
        Opcode::Gt(Val::Reg(1), Val::Reg(2), Val::Reg(1)),
        Opcode::Jf(Val::Reg(1), Val::Num(5636)),
        Opcode::Ret,
        Opcode::Add(Val::Reg(2), Val::Num(10666), Val::Num(956)),
    ];

    assert_eq!(instructions, expected)
}
