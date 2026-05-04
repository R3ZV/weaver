pub mod bpf_intf;
mod bpf_skel;
mod sched;
mod sched_settings;
#[rustfmt::skip]
mod bpf;

use anyhow::Result;
use clap::Parser;
use sched::Scheduler;
use sched_settings::Settings;
use std::mem::MaybeUninit;

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
