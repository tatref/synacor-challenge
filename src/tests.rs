use itertools::iproduct;

use crate::assembly::{Opcode, Val};
use crate::emulator::Vm;

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

    assert_eq!(Opcode::vec_to_machine_code(&prog), expected);

    Ok(())
}

#[test]
fn disassemble_function() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![
        Opcode::Set(Val::Reg(0), Val::Num(2)),
        Opcode::Eq(Val::Reg(0), Val::Reg(1), Val::Num(2)),
        Opcode::Jt(Val::Num(0), Val::Num(15)),
        Opcode::Ret,
        Opcode::Add(Val::Reg(2), Val::Num(1), Val::Num(2)),
        Opcode::Call(Val::Num(2)),
        Opcode::Call(Val::Num(2)),
        Opcode::Call(Val::Num(2)),
        Opcode::Call(Val::Num(2)),
        Opcode::Ret,
    ];
    let prog = Opcode::vec_to_machine_code(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&prog);

    let starting_ip = 0;
    let instructions = vm.disassemble_function(starting_ip)?;
    Vm::pretty_print_dis(&instructions);

    Ok(())
}

#[test]
fn patching_2125() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![Opcode::Call(Val::Num(2125))];
    let prog = Opcode::vec_to_machine_code(&prog);

    let mut vm = Vm::default();
    vm.load_program_from_mem(&prog);

    let mut vm1 = vm.clone();
    let mut vm2 = vm.clone();

    println!("vm1");
    for _ in 0..10 {
        vm1.step().unwrap();
    }

    println!("vm2");
    vm2.set_patching(true);
    vm2.step().unwrap();

    assert_eq!(vm1, vm2);

    Ok(())
}
#[test]
fn run_until_ret_2125() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![Opcode::Call(Val::Num(2125))];
    let prog = Opcode::vec_to_machine_code(&prog);

    for (reg0, reg1) in iproduct!(0..100, 0..100) {
        let mut vm = Vm::default();
        vm.load_program_from_mem(&prog);
        vm.set_register(0, reg0);
        vm.set_register(1, reg1);

        let mut vm1 = vm.clone();
        let mut vm2 = vm.clone();

        println!("vm1");
        let _instr = vm1.run_until_ret()?;

        println!("vm2");
        vm2.set_patching(true);
        let _instr = vm2.run_until_ret()?;

        assert_eq!(vm1, vm2);
    }

    Ok(())
}

#[test]
fn patching_3() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![
        Opcode::Call(Val::Num(3)),
        Opcode::Halt,
        Opcode::Set(Val::Reg(0), Val::Num(20)),
        Opcode::Ret,
    ];
    let prog = Opcode::vec_to_machine_code(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&prog);

    let x = vm.disassemble(0, 5)?;
    Vm::pretty_print_dis(&x);

    let mut vm1 = vm.clone();
    let mut vm2 = vm.clone();

    println!("vm1");
    vm1.step().unwrap();
    vm1.step().unwrap();
    vm1.step().unwrap();

    println!("vm2");
    vm2.set_patching(true);
    vm2.step().unwrap();

    assert_eq!(vm1, vm2);

    Ok(())
}

#[test]
fn run_until_ret_3() -> Result<(), Box<dyn std::error::Error>> {
    let prog = vec![
        Opcode::Call(Val::Num(3)),
        Opcode::Halt,
        Opcode::Set(Val::Reg(0), Val::Num(20)),
        Opcode::Ret,
    ];
    let prog = Opcode::vec_to_machine_code(&prog);

    let mut vm = Vm::new();
    vm.load_program_from_mem(&prog);

    let x = vm.disassemble(0, 5)?;
    Vm::pretty_print_dis(&x);
    println!();

    let mut vm1 = vm.clone();
    let mut vm2 = vm.clone();

    println!("vm1");
    vm1.run_until_ret().unwrap();
    //vm1.step().unwrap();
    //vm1.step().unwrap();
    //vm1.step().unwrap();

    println!("vm2");
    vm2.set_patching(true);
    vm2.run_until_ret().unwrap();
    //vm2.step().unwrap();

    assert_eq!(vm1, vm2);

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
