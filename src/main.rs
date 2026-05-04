pub mod bpf_intf;
mod bpf_skel;
mod sched;
#[rustfmt::skip]
mod bpf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use sched::Scheduler;
use std::fmt;
use std::mem::MaybeUninit;

#[derive(ValueEnum, Clone, Debug, Default)]
enum DecayFunction {
    Continous,

    #[default]
    Discrete,
}

#[derive(ValueEnum, Clone, Debug, Default)]
enum DispatchStrategy {
    /// Send twice as many tasks as are cpus on the machine per dispatch
    Batch,

    /// Send a single task to the cpu per dispatch
    #[default]
    DripFeed,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Settings {
    #[arg(short, long, default_value_t)]
    decay_function: DecayFunction,

    #[arg(short, long, default_value_t)]
    dispatch_strategy: DispatchStrategy,
}

impl fmt::Display for DecayFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecayFunction::Discrete => write!(f, "discrete"),
            DecayFunction::Continous => write!(f, "continous"),
        }
    }
}

impl fmt::Display for DispatchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchStrategy::Batch => write!(f, "batch"),
            DispatchStrategy::DripFeed => write!(f, "drip-feed"),
        }
    }
}

fn main() -> Result<()> {
    let settings = Settings::parse();

    // ASCII art from:
    // https://emojicombos.com/spider-ascii-art
    let art = r#"
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⢠⣴⣿⣿⣿⣷⣼⣿⠀⣴⠾⠷⠶⠦⡄⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⢠⡤⢶⣦⣾⣿⣿⣿⣿⣿⣿⣿⠀⣿⣶⣶⣦⣄⠳⣤⣤⠄⠀⠀⠀
⠀⠀⠀⢀⣼⣳⡿⢻⣿⣿⣿⣿⣿⣿⣿⣿⣶⣿⣿⣗⠈⠙⠻⣶⣄⡀⠀⠀⠀
⠀⠀⠀⣰⠿⠁⢀⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⡄⠀⠀⠈⠳⣤⠀⠀
⠀⠀⢀⡟⠀⢰⣿⠟⠻⢿⣿⣿⣿⣿⣿⣿⣿⣿⠉⠁⠈⠻⣶⣄⠀⠀⠈⠛⢦
⠀⣀⡼⠃⠀⣼⡟⠀⠀⢸⣿⡿⠉⣿⡿⠿⠛⣿⡄⠀⠀⠀⠙⠿⣆⠀⠀⠀⠈
⠈⠁⠀⠀⢸⡟⠀⠀⠀⢸⣿⠀⠀⣿⠁⠀⠀⠈⠃⠀⠀⠀⠀⠀⠘⢷⡄⠀⠀
⠀⠀⠀⠀⣼⠃⠀⠀⠀⢸⡟⠀⠀⡿⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⢿⡆⠀
⠀⠀⠀⣠⡏⠀⠀⠀⠀⣼⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠻⠃⠀⠀⠀⠀⣻⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠻⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
"#;

    println!("{}", art);
    println!("Running weaver...");
    let mut open_object = MaybeUninit::uninit();
    loop {
        let mut sched = Scheduler::init(&mut open_object, &settings)?;
        if !sched.run()?.should_restart() {
            break;
        }
    }

    Ok(())
}
