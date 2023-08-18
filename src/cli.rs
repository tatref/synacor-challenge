use crate::emulator::*;
//use clap::{App, AppSettings, Arg, SubCommand};
use clap::{builder::RangedU64ValueParser, Arg, Command};

#[derive(Debug)]
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
            .subcommand(Command::new("vm"))
            .subcommand(Command::new("run").alias("r"))
            .subcommand(Command::new("input").alias("i").arg(Arg::new("line")))
            .subcommand(
                Command::new("snapshot")
                    .alias("snap")
                    .subcommand(
                        Command::new("take")
                            .alias("t")
                            .arg(Arg::new("name").required(true)),
                    )
                    .subcommand(
                        Command::new("restore").alias("r").arg(
                            Arg::new("idx")
                                .required(true)
                                .value_parser(RangedU64ValueParser::<usize>::new()),
                        ),
                    )
                    .subcommand(Command::new("list").alias("l")),
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

    fn take_snapshot(&mut self, name: &str) {
        self.snapshots.push(Snapshot {
            name: name.to_string(),
            vm: self.vm.clone(),
        });
    }

    fn remove_snapshot(&mut self, idx: usize) {
        if idx < self.snapshots.len() {
            self.snapshots.remove(idx);
            println!("Removed {}", idx);
        } else {
            println!("Can remove snapshot {}", idx);
        }
    }

    fn restore_snapshot(&mut self, idx: usize) {
        if idx < self.snapshots.len() {
            let snap = &self.snapshots[idx];
            self.vm = snap.vm.clone();
            println!("Restored {}", idx);
        } else {
            println!("Can revert snapshot {}", idx);
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
                    return Ok(());
                }
                Err(e) => {
                    println!("Invalid command, tried feeding, but didn't work either");
                    return Err(e);
                }
            },
        };

        match args.subcommand() {
            Some(("run", sub)) => self.vm.run(),
            Some(("input", sub)) => self.vm.feed(sub.get_one::<String>("line").unwrap())?,
            Some(("vm", sub)) => {
                println!("{:?}", self.vm);
            }
            Some(("snapshot", sub)) => match sub.subcommand() {
                Some(("take", subsub)) => {
                    let name = subsub.get_one::<String>("name").unwrap();
                    self.take_snapshot(name);
                }
                Some(("restore", subsub)) => {
                    let idx = *subsub.get_one::<usize>("idx").unwrap();
                    self.restore_snapshot(idx);
                }
                Some(("remove", subsub)) => {
                    let idx = *subsub.get_one("idx").unwrap();
                    self.remove_snapshot(idx);
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
                    self.vm.step();
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
