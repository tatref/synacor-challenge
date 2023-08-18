#[test]
fn load_program_from_file() -> Result<(), ()> {
    use super::Vm;

    let f = "challenge.bin";
    let mut vm = Vm::default();
    vm.load_program_from_file(f)
}

#[test]
fn load_program_from_mem() -> Result<(), ()> {
    use super::Vm;

    let mut vm = Vm::default();
    let program = [9, 32768, 32769, 4, 19, 32768];
    vm.load_program_from_mem(&program);

    Ok(())
}
