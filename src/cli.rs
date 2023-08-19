use crate::{emulator::*, solver::GameSolver};
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

    solver: GameSolver,
}

impl Cli {
    pub fn new(vm: Vm) -> Self {
        let cli = Command::new("cli")
            .subcommand_required(true)
            .no_binary_name(true)
            .subcommand(Command::new("helpme"))
            .subcommand(
                Command::new("vm").subcommand(
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
            .subcommand(Command::new("run").alias("r"))
            .subcommand(Command::new("input").alias("i").arg(Arg::new("line")))
            .subcommand(
                Command::new("solver")
                    .subcommand(Command::new("explore"))
                    .subcommand(Command::new("bruteforce")),
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

        let solver = GameSolver::new();

        Self {
            cli,
            vm,
            snapshots: Vec::new(),

            solver,
        }
    }

    fn get_snap_by_name(&self, name: &str) -> Option<&Snapshot> {
        self.snapshots
            .iter()
            .filter(|snap| snap.name == name)
            .next()
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

        self.snapshots.push(snap);
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
            Some(("run", sub)) => {
                self.vm.run();
                println!("{}", self.vm.get_messages().last().unwrap());
            }
            Some(("input", sub)) => {
                self.vm
                    .feed(sub.get_one::<String>("line").unwrap_or(&"".to_string()))?;
                println!("{}", self.vm.get_messages().last().unwrap());
            }
            Some(("vm", sub)) => match sub.subcommand() {
                Some(("register", subsub)) => match subsub.subcommand() {
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
                Some(("explore", subsub)) => {
                    let solver = GameSolver::new();
                    solver.explore_maze(&self.vm, "Twisty passages");
                }
                Some(("bruteforce", subsub)) => {
                    let solver = GameSolver::new();
                    solver.bruteforce_teleporter(&self.vm);
                }
                Some((_, _)) => return Err("unreachable?".into()),
                None => (),
            },
            Some(("snap", sub)) => match sub.subcommand() {
                Some(("dump", subsub)) => {
                    let name = subsub.get_one::<String>("name").unwrap();
                    let dump_path = subsub.get_one::<String>("dump_path").unwrap();
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
                Some(("take", subsub)) => {
                    let name = subsub.get_one::<String>("name").unwrap();
                    self.take_snapshot(name);
                }
                Some(("restore", subsub)) => {
                    let name = subsub.get_one::<String>("name").unwrap();
                    self.restore_snapshot(name);
                }
                Some(("remove", subsub)) => {
                    let name = subsub.get_one::<String>("name").unwrap();
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
                for i in 0..count {
                    let _ = self.vm.step();
                }
            }
            Some(("helpme", _)) => {
                self.cli.print_long_help().unwrap();
            }
            Some((x, y)) => unimplemented!("Unknown command {x:?}"),
            None => (),
        }

        Ok(())
    } // end fn parse_command
}
