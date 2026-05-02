mod bpf_skel;
pub use bpf_skel::*;
pub mod bpf_intf;

#[rustfmt::skip]
mod bpf;
use std::collections::{BinaryHeap, HashMap};
use std::mem::MaybeUninit;
use std::time::SystemTime;

use anyhow::Result;
use bpf::*;
use libbpf_rs::OpenObject;
use scx_utils::UserExitInfo;
use scx_utils::libbpf_clap_opts::LibbpfOpts;

const MIN_SLICE_NS: u64 = 1_000_000;
const RUNTIME_NS: u64 = 15_000_000;
const LC_HALF_LIFE_NS: f64 = 5_000_000.0;
const LC_WAKEUP_BOOST: f64 = 100.0;

struct Task {
    inner: QueuedTask,
    v_deadline: u64,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.v_deadline == other.v_deadline
    }
}

impl Eq for Task {}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.v_deadline.cmp(&self.v_deadline)
    }
}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

struct TaskMetadata {
    // latency criticality
    lc: f64,
    vtime: f64,
    prev_decay_ns: u64,
}

impl TaskMetadata {
    fn new(now_ns: u64) -> Self {
        Self {
            vtime: 0.0,
            lc: 0.0,
            prev_decay_ns: now_ns,
        }
    }

    fn decay(&mut self, now_ns: u64) {
        let delta_ns = now_ns.saturating_sub(self.prev_decay_ns);
        if delta_ns > 0 {
            let decay_factor = (0.5_f64).powf(delta_ns as f64 / LC_HALF_LIFE_NS);
            self.lc *= decay_factor;
        }
        self.prev_decay_ns = now_ns;
    }
}

struct Scheduler<'a> {
    bpf: BpfScheduler<'a>,
    tasks: BinaryHeap<Task>,
    meta: HashMap<i32, TaskMetadata>,
}

impl<'a> Scheduler<'a> {
    fn init(open_object: &'a mut MaybeUninit<OpenObject>) -> Result<Self> {
        let open_opts = LibbpfOpts::default();
        let bpf = BpfScheduler::init(
            open_object,
            open_opts.clone().into_bpf_open_opts(),
            0,          // exit_dump_len (buffer size of exit info, 0 = default)
            false,      // partial (false = include all tasks)
            false,      // debug (false = debug mode off)
            true,       // builtin_idle (true = allow BPF to use idle CPUs if available)
            true,       // numa_local (true = allow BPF to use a NUMA-local idle CPU
            RUNTIME_NS, // default time slice (for tasks automatically dispatched by the backend)
            "weaver",   // name of the scx ops
        )?;
        Ok(Self {
            bpf,
            tasks: BinaryHeap::new(),
            meta: HashMap::new(),
        })
    }

    fn consume_wake_events(&mut self) {
        while let Ok(Some(event)) = self.bpf.dequeue_wakeup_event() {
            let now = Self::now();
            let entry = self
                .meta
                .entry(event.wakee_pid)
                .or_insert(TaskMetadata::new(now));
            entry.decay(now);
            entry.lc += LC_WAKEUP_BOOST;
        }
    }

    fn retrieve_new_tasks(&mut self) {
        while let Ok(Some(task)) = self.bpf.dequeue_task() {
            let now = Self::now();
            let weight = (task.weight as f64).max(1.0);
            let exec_runtime = task.exec_runtime as f64;

            let entry = self.meta.entry(task.pid).or_insert(TaskMetadata::new(now));
            entry.decay(now);
            entry.vtime += exec_runtime / weight;

            let v_deadline = entry.vtime + (exec_runtime / (weight + entry.lc));
            self.tasks.push(Task {
                inner: task,
                v_deadline: v_deadline as u64,
            })
        }
    }

    fn dispatch_tasks(&mut self) {
        let nr_waiting = self.tasks.len() as u64;
        while let Some(task) = self.tasks.pop() {
            let mut dispatched_task = DispatchedTask::new(&task.inner);
            let cpu = self.bpf.select_cpu(
                dispatched_task.pid,
                dispatched_task.cpu,
                dispatched_task.flags,
            );
            dispatched_task.cpu = if cpu >= 0 { cpu } else { RL_CPU_ANY };
            let exec_slice = RUNTIME_NS / (nr_waiting + 1);
            dispatched_task.slice_ns = exec_slice.max(MIN_SLICE_NS);
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

        eprintln!(
            "[WEAVER LOG]: user={} kernel={} cancel={} bounce={} fail={} cong={}",
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
            .as_nanos() as u64
    }

    fn run(&mut self) -> Result<UserExitInfo> {
        let mut prev_ts = Self::now();
        let stat_interval_ns = 1_000_000_000;

        while !self.bpf.exited() {
            self.consume_wake_events();
            self.retrieve_new_tasks();

            let curr_ts = Self::now();
            if curr_ts.saturating_sub(prev_ts) > stat_interval_ns {
                self.print_stats();
                prev_ts = curr_ts;
            }

            self.dispatch_tasks();
        }
        self.bpf.shutdown_and_report()
    }
}

fn main() -> Result<()> {
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
        let mut sched = Scheduler::init(&mut open_object)?;
        if !sched.run()?.should_restart() {
            break;
        }
    }

    Ok(())
}
