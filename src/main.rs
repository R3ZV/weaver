mod bpf_skel;
pub use bpf_skel::*;
pub mod bpf_intf;

#[rustfmt::skip]
mod bpf;
use std::mem::MaybeUninit;
use std::time::SystemTime;

use anyhow::Result;
use bpf::*;
use libbpf_rs::OpenObject;
use scx_utils::UserExitInfo;
use scx_utils::libbpf_clap_opts::LibbpfOpts;

const SLICE_NS: u64 = 5_000_000;

struct Scheduler<'a> {
    bpf: BpfScheduler<'a>,
}

impl<'a> Scheduler<'a> {
    fn init(open_object: &'a mut MaybeUninit<OpenObject>) -> Result<Self> {
        let open_opts = LibbpfOpts::default();
        let bpf = BpfScheduler::init(
            open_object,
            open_opts.clone().into_bpf_open_opts(),
            0,        // exit_dump_len (buffer size of exit info, 0 = default)
            false,    // partial (false = include all tasks)
            false,    // debug (false = debug mode off)
            true,     // builtin_idle (true = allow BPF to use idle CPUs if available)
            SLICE_NS, // default time slice (for tasks automatically dispatched by the backend)
            "weaver", // name of the scx ops
        )?;
        Ok(Self { bpf })
    }

    fn dispatch_tasks(&mut self) {
        let nr_waiting = *self.bpf.nr_queued_mut();

        while let Ok(Some(task)) = self.bpf.dequeue_task() {
            let mut dispatched_task = DispatchedTask::new(&task);
            let cpu = self.bpf.select_cpu(task.pid, task.cpu, task.flags);
            dispatched_task.cpu = if cpu >= 0 { cpu } else { RL_CPU_ANY };

            dispatched_task.slice_ns = SLICE_NS / (nr_waiting + 1);
            self.bpf.dispatch_task(&dispatched_task).unwrap();
        }
        self.bpf.notify_complete(0);
    }

    fn print_stats(&mut self) {
        // *self.bpf.nr_online_cpus_mut();       // amount of online CPUs
        // *self.bpf.nr_running_mut();           // amount of currently running tasks
        // *self.bpf.nr_queued_mut();            // amount of tasks queued to be scheduled
        // *self.bpf.nr_scheduled_mut();         // amount of tasks managed by the user-space scheduler
        //
        // *self.bpf.nr_user_dispatches_mut();   // amount of user-space dispatches
        // *self.bpf.nr_kernel_dispatches_mut(); // amount of kernel dispatches
        // *self.bpf.nr_cancel_dispatches_mut(); // amount of cancelled dispatches
        // *self.bpf.nr_bounce_dispatches_mut(); // amount of bounced dispatches
        // *self.bpf.nr_failed_dispatches_mut(); // amount of failed dispatches
        // *self.bpf.nr_sched_congested_mut();   // amount of scheduler congestion events

        let nr_user_dispatches = *self.bpf.nr_user_dispatches_mut();
        let nr_kernel_dispatches = *self.bpf.nr_kernel_dispatches_mut();
        let nr_cancel_dispatches = *self.bpf.nr_cancel_dispatches_mut();
        let nr_bounce_dispatches = *self.bpf.nr_bounce_dispatches_mut();
        let nr_failed_dispatches = *self.bpf.nr_failed_dispatches_mut();
        let nr_sched_congested = *self.bpf.nr_sched_congested_mut();

        println!(
            "user={} kernel={} cancel={} bounce={} fail={} cong={}",
            nr_user_dispatches,
            nr_kernel_dispatches,
            nr_cancel_dispatches,
            nr_bounce_dispatches,
            nr_failed_dispatches,
            nr_sched_congested,
        );
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn run(&mut self) -> Result<UserExitInfo> {
        let mut prev_ts = Self::now();

        while !self.bpf.exited() {
            self.dispatch_tasks();

            let curr_ts = Self::now();
            if curr_ts > prev_ts {
                self.print_stats();
                prev_ts = curr_ts;
            }
        }
        self.bpf.shutdown_and_report()
    }
}

fn main() -> Result<()> {
    let mut open_object = MaybeUninit::uninit();
    loop {
        let mut sched = Scheduler::init(&mut open_object)?;
        if !sched.run()?.should_restart() {
            break;
        }
    }

    Ok(())
}
