use crate::assembly::Opcode;
use crate::{emulator::*, solver::GameSolver};
use clap::builder::BoolishValueParser;
//use clap::{App, AppSettings, Arg, SubCommand};
use clap::{builder::RangedU64ValueParser, Arg, Command};

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    name: String,
    vm: Vm,
}

pub struct Cli {
    pub cli: Command,

    pub vm: Vm,
    pub snapshots: Vec<Snapshot>,
}

impl Cli {
    pub fn new(vm: Vm) -> Self {
        let cli = Command::new("cli")
            .subcommand_required(true)
            .no_binary_name(true)
            .subcommand(Command::new("helpme"))
            .subcommand(
                Command::new("bp")
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
                    .arg(Arg::new("opcode"))
                    .arg(Arg::new("offset").value_parser(RangedU64ValueParser::<usize>::new())),
            )
            .subcommand(
                Command::new("dis")
                    .subcommand(
                        Command::new("at")
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
                        Command::new("fn").arg(
                            Arg::new("from")
                                .required(true)
                                .value_parser(RangedU64ValueParser::<usize>::new()),
                        ),
                    ),
            )
            .subcommand(
                Command::new("vm")
                    .subcommand(
                        Command::new("patch").arg(
                            Arg::new("patch")
                                .required(true)
                                .value_parser(BoolishValueParser::new()),
                        ),
                    )
                    .subcommand(
                        Command::new("register").subcommand(
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
            .subcommand(Command::new("input").alias("i").arg(Arg::new("line")))
            .subcommand(
                Command::new("solver")
                    .subcommand(Command::new("explore"))
                    .subcommand(Command::new("teleporter")),
            )
            .subcommand(
                Command::new("snap")
                    .subcommand(Command::new("load").arg(Arg::new("dump_path").required(true)))
                    .subcommand(
                        Command::new("dump")
                            .arg(Arg::new("name").required(true))
                            .arg(Arg::new("dump_path").required(true)),
                    )
                    .subcommand(Command::new("take").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("remove").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("restore").arg(Arg::new("name").required(true)))
                    .subcommand(Command::new("list")),
            )
            .subcommand(
                Command::new("step").alias("s").arg(
                    Arg::new("count")
                        .value_parser(RangedU64ValueParser::<u32>::new())
                        .default_value("1"),
                ),
            );

        Self {
            cli,
            vm,
            snapshots: Vec::new(),
        }
    }

    fn get_snap_by_name(&self, name: &str) -> Option<&Snapshot> {
        self.snapshots.iter().find(|snap| snap.name == name)
    }

    fn dump_snapshot(&mut self, name: &str, dump_path: &str) {
        match self.get_snap_by_name(name) {
            Some(snap) => {
                let mut f = std::fs::File::create(dump_path).unwrap();
                serde_json::to_writer(&mut f, &snap).unwrap();
            }
            None => println!("Snap not found"),
        }
    }

    fn load_snapshot(&mut self, dump_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let f = std::fs::File::open(dump_path)?;
        let snap: Snapshot = serde_json::from_reader(f)?;
        let name = snap.name.clone();

        match self.snapshots.iter().find(|s| s.name == name) {
            Some(_) => (),
            None => self.snapshots.push(snap),
        };
        self.restore_snapshot(&name);

        Ok(())
    }

    fn take_snapshot(&mut self, name: &str) {
        self.snapshots.push(Snapshot {
            name: name.to_string(),
            vm: self.vm.clone(),
        });
    }

    fn remove_snapshot(&mut self, name: &str) {
        let mut idx = None;
        for (i, snap) in self.snapshots.iter().enumerate() {
            if snap.name == name {
                idx = Some(i);
                break;
            }
        }

        match idx {
            Some(idx) => {
                self.snapshots.remove(idx);
            }
            None => println!("Not found"),
        }
    }

    fn restore_snapshot(&mut self, name: &str) {
        match self.get_snap_by_name(name) {
            Some(snap) => {
                self.vm = snap.vm.clone();
            }
            None => println!("Snap not found"),
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
            Err(_) => match self.vm.feed(input_line) {
                Ok(_) => {
                    self.vm.run();
                    println!("{}", self.vm.get_messages().last().unwrap());
                    return Ok(());
                }
                Err(e) => {
                    println!("Invalid command, tried feeding, but didn't work either");
                    return Err(e);
                }
            },
        };

        match args.subcommand() {
            Some(("run", _sub)) => {
                self.vm.run();
                if let VmState::WaitingForInput = self.vm.get_state() {
                    println!("{}", self.vm.get_messages().last().unwrap());
                }
            }
            Some(("input", sub)) => {
                self.vm
                    .feed(sub.get_one::<String>("line").unwrap_or(&"".to_string()))?;
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
                    let instructions = self.vm.disassemble_function(from)?;

                    Vm::pretty_print_dis(&instructions);
                }
                Some(_) => (),

                None => (),
            },
            Some(("vm", sub)) => match sub.subcommand() {
                Some(("patch", sub)) => {
                    let patching = *sub.get_one::<bool>("patch").unwrap();
                    self.vm.set_patching(patching);
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
            Some(("snap", sub)) => match sub.subcommand() {
                Some(("dump", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    let dump_path = sub.get_one::<String>("dump_path").unwrap();
                    self.dump_snapshot(name, &format!("snaps/{}", dump_path));
                }
                Some(("load", subsub)) => {
                    let dump_path = subsub.get_one::<String>("dump_path").unwrap();
                    self.load_snapshot(&format!("snaps/{}", dump_path))?;
                    println!(
                        "Last message was:\n{}",
                        self.vm.get_messages().last().unwrap()
                    );
                }
                Some(("take", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.take_snapshot(name);
                }
                Some(("restore", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.restore_snapshot(name);
                }
                Some(("remove", sub)) => {
                    let name = sub.get_one::<String>("name").unwrap();
                    self.remove_snapshot(name);
                }
                Some(("list", _)) => {
                    println!("{} snapshots:", self.snapshots.len());
                    for (idx, snap) in self.snapshots.iter().enumerate() {
                        println!("{} {:?}", idx, snap.name);
                    }
                }
                _ => {
                    let name = format!("{:03}", self.snapshots.len());
                    self.take_snapshot(&name);
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
            Some(("helpme", _)) => {
                self.cli.print_long_help().unwrap();
            }
            Some((x, _sub)) => unimplemented!("Unknown command {x:?}"),
            None => (),
        }

        Ok(())
    } // end fn parse_command
}
