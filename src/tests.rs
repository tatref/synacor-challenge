use crate::emulator::{Opcode, Val, Vm};

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
    let instructions = vm.disassemble_function(starting_ip);
    let mut last: Option<(usize, Opcode)> = None;
    for &(offset, opcode) in instructions.iter() {
        if let Some((previous_offset, previous_opcode)) = last {
            if previous_opcode.size() + previous_offset < offset {
                println!("[...]");
            }
        }

        println!("{}: {:?}", starting_ip + offset, opcode);
        last = Some((offset, opcode));
    }

    panic!();

    Ok(())
}
