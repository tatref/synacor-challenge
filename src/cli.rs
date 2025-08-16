use std::collections::HashMap;
use std::path::Path;

use crate::assembly::Opcode;
use crate::{emulator::*, solver::GameSolver};
use clap::builder::BoolishValueParser;
//use clap::{App, AppSettings, Arg, SubCommand};
use clap::{builder::RangedU64ValueParser, Arg, Command};

use itertools::Itertools;

pub struct Cli {
    pub cli: Command,

    pub vm: Vm,
    pub saved_states: HashMap<String, Vm>,
}

impl Cli {
    pub fn new(vm: Vm) -> Self {
        let cli = Command::new("cli")
            .subcommand_required(true)
            .no_binary_name(true)
            .subcommand(
                Command::new("bp")
                    .about("Breakpoints")
                    .subcommand(Command::new("list"))
                    .subcommand(
                        Command::new("set").arg(
                            Arg::new("offset").value_parser(RangedU64ValueParser::<usize>::new()),
                        ),
                    )
                    .subcommand(Command::new("unset").arg(
                        Arg::new("offset").value_parser(RangedU64ValueParser::<usize>::new()),
                    )),
            )
            .subcommand(
                Command::new("patch")
                    .about("Patch opcodes")
                    .arg(Arg::new("opcode"))
                    .arg(Arg::new("offset").value_parser(RangedU64ValueParser::<usize>::new())),
            )
            .subcommand(
                Command::new("dis")
                    .about("Disassembler")
                    .subcommand(
                        Command::new("at")
                            .about("Disassemble at memory offset")
                            .arg(
                                Arg::new("from")
                                    .required(true)
                                    .value_parser(RangedU64ValueParser::<usize>::new()),
                            )
                            .arg(
                                Arg::new("count")
                                    .required(true)
                                    .value_parser(RangedU64ValueParser::<usize>::new()),
                            ),
                    )
                    .subcommand(
                        Command::new("fn").about("Disassemble function").arg(
                            Arg::new("from")
                                .required(true)
                                .value_parser(RangedU64ValueParser::<usize>::new()),
                        ),
                    ),
            )
            .subcommand(
                Command::new("vm")
                    .subcommand(
                        Command::new("fn_patching").about("Function patching").arg(
                            Arg::new("patch")
                                .required(true)
                                .value_parser(BoolishValueParser::new()),
                        ),
                    )
                    .subcommand(
                        Command::new("register").about("Edit register").subcommand(
                            Command::new("set")
                                .arg(
                                    Arg::new("register")
                                        .required(true)
                                        .value_parser(RangedU64ValueParser::<usize>::new()),
                                )
                                .arg(
                                    Arg::new("value")
                                        .required(true)
                                        .value_parser(RangedU64ValueParser::<u16>::new()),
                                ),
                        ),
                    ),
            )
            .subcommand(
                Command::new("mem")
                    .about("Search memory")
                    .subcommand(Command::new("init"))
                    .subcommand(Command::new("list"))
                    .subcommand(
                        Command::new("get").arg(
                            Arg::new("offset").value_parser(RangedU64ValueParser::<usize>::new()),
                        ),
                    )
                    .subcommand(
                        Command::new("set")
                            .arg(
                                Arg::new("offset")
                                    .value_parser(RangedU64ValueParser::<usize>::new()),
                            )
                            .arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ),
                    )
                    .subcommand(
                        Command::new("filter")
                            .alias("f")
                            .subcommand(Command::new("=").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ))
                            .subcommand(Command::new("!=").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ))
                            .subcommand(Command::new(">").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ))
                            .subcommand(Command::new(">=").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ))
                            .subcommand(Command::new("<").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            ))
                            .subcommand(Command::new("<=").arg(
                                Arg::new("value").value_parser(RangedU64ValueParser::<u16>::new()),
                            )),
                    ),
            )
            .subcommand(Command::new("run").alias("r"))
            .subcommand(
                Command::new("input").about("Feed input").alias("i").arg(
                    Arg::new("line")
                        .num_args(1..)
                        .trailing_var_arg(true)
                        .allow_hyphen_values(true),
                ),
            )
            .subcommand(
                Command::new("solver")
                    .about("Challenge specific solvers")
                    .subcommand(Command::new("explore"))
                    .subcommand(Command::new("teleporter")),
            )
            .subcommand(
                Command::new("state")
                    .about("Emulator save/load state")
                    .subcommand(Command::new("diskload").arg(Arg::new("dump_path").required(true)))
                    .subcommand(
                        Command::new("disksave")
                            .arg(Arg::new("name").required(true))
                            .arg(Arg::new("dump_path").required(true)),
                    )
                    .subcommand(Command::new("save").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("remove").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("load").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("list")),
            )
            .subcommand(
                Command::new("step")
                    .about("Execute N instructions")
                    .alias("s")
                    .arg(
                        Arg::new("count")
                            .value_parser(RangedU64ValueParser::<u32>::new())
                            .default_value("1"),
                    ),
            );

        Self {
            cli,
            vm,
            saved_states: Default::default(),
        }
    }

    fn get_state_by_name(&self, name: &str) -> Option<&Vm> {
        self.saved_states.get(name)
    }

    fn save_state_to_disk(&mut self, name: &str, dump_path: &str) {
        match self.get_state_by_name(name) {
            Some(state) => {
                let mut f = std::fs::File::create(dump_path).unwrap();
                serde_json::to_writer(&mut f, &state).unwrap();
            }
            None => println!("State not found"),
        }
    }

    fn load_state_from_disk<P: AsRef<Path>>(
        &mut self,
        dump_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let f = std::fs::File::open(&dump_path)?;
        let state: Vm = serde_json::from_reader(f)?;
        let name = dump_path
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        self.saved_states.insert(name.clone(), state);

        // load state to current Vm
        self.load_state(&name);

        Ok(())
    }

    fn save_state(&mut self, name: &str) {
        self.saved_states.insert(name.to_string(), self.vm.clone());
    }

    fn remove_state(&mut self, name: &str) {
        self.saved_states.remove(name);
    }

    fn load_state(&mut self, name: &str) {
        match self.get_state_by_name(name) {
            Some(state) => {
                self.vm = state.clone();
            }
            None => println!("State not found"),
        }
    }

    pub fn parse_command(&mut self, input_line: &str) -> Result<(), Box<dyn std::error::Error>> {
        if input_line.split_whitespace().next().is_none() {
            // empy command
            return Ok(());
        }

        let argv = input_line.split_whitespace();
        let args = match self.cli.clone().try_get_matches_from(argv.clone()) {
            Ok(args) => args,
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
                e.print().unwrap();
                return Ok(());
            }
            Err(e) => {
                e.print().unwrap();
                return Ok(());
                //return Err(Box::new(e));
            }
        };

        match args.subcommand() {
            Some(("run", _sub)) => {
                let _executed = self
                    .vm
                    .run_until(StopVmState::new(&[VmState::WaitingForInput]))
                    .unwrap();

                if let VmState::WaitingForInput = self.vm.get_state() {
                    println!("{}", self.vm.get_messages().last().unwrap());
                }
            }
            Some(("input", sub)) => {
                let input = match sub.get_many::<String>("line") {
                    Some(x) => x,
                    None => return Ok(()),
                };
                let input: String = input.cloned().join(" ");
                self.vm.feed(&input)?;
                //self.vm.run();
                let _executed = self
                    .vm
                    .run_until(StopVmState::new(&[VmState::WaitingForInput]))
                    .unwrap();
                println!("{}", self.vm.get_messages().last().unwrap());
            }
            Some(("patch", sub)) => {
                let opcode = sub.get_one::<String>("opcode").unwrap();
                let opcode: Opcode = opcode.parse()?;
                let offset = *sub.get_one::<usize>("offset").unwrap();

                self.vm.patch(opcode, offset);
            }
            Some(("mem", sub)) => match sub.subcommand() {
                Some(("init", _sub)) => {
                    self.vm.scanmem_init();
                }
                Some(("list", _sub)) => {
                    self.vm.scanmem_list();
                }
                Some(("get", sub)) => {
                    let offset = *sub.get_one::<usize>("offset").unwrap();
                    self.vm.mem_get(offset);
                }
                Some(("set", sub)) => {
                    let offset = *sub.get_one::<usize>("offset").unwrap();
                    let value = *sub.get_one::<u16>("value").unwrap();
                    self.vm.mem_set(offset, value);
                }
                Some(("filter", sub)) => {
                    if let Some((filter, sub)) = sub.subcommand() {
                        let value = sub.get_one::<u16>("value").copied();
                        self.vm.scanmem_filter(filter, value);
                    }
                }
                Some(_) => println!("Unknown command"),
                None => (),
            },
            Some(("bp", sub)) => match sub.subcommand() {
                Some(("list", _sub)) => {
                    for &bp in self.vm.get_breakpoints() {
                        match self.vm.disassemble(bp, 1) {
                            Ok(x) => Vm::pretty_print_dis(&x),
                            Err(e) => println!("{}: {}", bp, e),
                        }
                    }
                }
                Some(("set", sub)) => {
                    let offset = *sub.get_one::<usize>("offset").unwrap();
                    self.vm.set_breakpoint(offset);
                }
                Some(("unset", sub)) => {
                    let offset = *sub.get_one::<usize>("offset").unwrap();
                    self.vm.unset_breakpoint(offset);
                }
                Some(_) => (),

                None => (),
            },
            Some(("dis", sub)) => match sub.subcommand() {
                Some(("at", sub)) => {
                    let from = *sub.get_one::<usize>("from").unwrap();
                    let count = *sub.get_one::<usize>("count").unwrap();

                    let instructions = self.vm.disassemble(from, count)?;
                    for (ip, instr) in instructions.iter() {
                        println!("{}: {:?}", ip, instr);
                    }
                }
                Some(("fn", sub)) => {
                    let from = *sub.get_one::<usize>("from").unwrap();
                    let function = self.vm.disassemble_function(from)?;
                    function.pretty_print();
                }
                Some(_) => (),

                None => (),
            },
            Some(("vm", sub)) => match sub.subcommand() {
                Some(("fn_patching", sub)) => {
                    let patching = *sub.get_one::<bool>("patch").unwrap();
                    self.vm.set_fn_patching(patching);
                    match patching {
                        true => println!("fn_patching: ✔️"),
                        false => println!("fn_patching: ❌"),
                    }
                }
                Some(("register", sub)) => match sub.subcommand() {
                    Some(("set", sub)) => {
                        let reg = *sub.get_one::<usize>("register").unwrap();
                        let value = *sub.get_one::<u16>("value").unwrap();

                        self.vm.set_register(reg, value);
                    }
                    Some(_) => (),
                    None => (),
                },
                Some((_, _)) => return Err("unreachable?".into()),
                None => println!("{:?}", self.vm),
            },

            Some(("solver", sub)) => match sub.subcommand() {
                Some(("explore", _sub)) => {
                    GameSolver::explore_maze(&self.vm);
                }
                Some(("teleporter", _sub)) => {
                    GameSolver::trace_teleporter(&self.vm);
                }
                Some((_, _)) => return Err("unreachable?".into()),
                None => (),
            },
            Some(("state", sub)) => match sub.subcommand() {
                Some(("disksave", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    let dump_path = sub.get_one::<String>("dump_path").unwrap();
                    self.save_state_to_disk(name, &format!("saved_states/{}", dump_path));
                }
                Some(("diskload", subsub)) => {
                    let dump_path = subsub.get_one::<String>("dump_path").unwrap();
                    self.load_state_from_disk(&format!("saved_states/{}", dump_path))?;
                    println!(
                        "Last message was:\n{}",
                        self.vm.get_messages().last().unwrap()
                    );
                }
                Some(("save", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.save_state(name);
                    println!("Saved");
                }
                Some(("load", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.load_state(name);
                }
                Some(("remove", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.remove_state(name);
                }
                Some(("list", _)) => {
                    println!("{} saved states:", self.saved_states.len());
                    for (name, _state) in self.saved_states.iter() {
                        println!("{:?}", name);
                    }
                }
                _ => {
                    println!("{} saved states:", self.saved_states.len());
                    for (name, _state) in self.saved_states.iter() {
                        println!("{:?}", name);
                    }
                }
            },
            Some(("step", sub)) => {
                let count: u32 = *sub.get_one("count").unwrap();
                for _ in 0..count {
                    match self.vm.step() {
                        Ok(_) => (),
                        Err(e) => println!("{}", e),
                    }
                }
            }
            Some((x, _sub)) => unimplemented!("Unknown command {x:?}"),
            None => (),
        }

        Ok(())
    } // end fn parse_command
}
