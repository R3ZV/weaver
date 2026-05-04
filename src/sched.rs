use std::collections::{BinaryHeap, HashMap};
use std::mem::MaybeUninit;
use std::time::SystemTime;

use crate::bpf::*;
use crate::sched_settings::{DecayFunction, DispatchStrategy, Settings};
use anyhow::Result;
use libbpf_rs::OpenObject;
use scx_utils::UserExitInfo;
use scx_utils::libbpf_clap_opts::LibbpfOpts;

const MIN_SLICE_NS: u64 = 1_000_000;
const RUNTIME_NS: u64 = 15_000_000;
const LC_WAKEUP_BOOST: f64 = 100.0;

pub struct Scheduler<'a> {
    bpf: BpfScheduler<'a>,
    tasks: BinaryHeap<Task>,
    meta: HashMap<i32, TaskMetadata>,
    system_vtime: u64,
    settings: &'a Settings,
}

struct Task {
    inner: QueuedTask,
    v_deadline: u64,
}

struct TaskMetadata {
    // latency criticality
    lc: f64,
    vtime: u64,
    prev_decay_ns: u64,
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

impl TaskMetadata {
    fn new(now_ns: u64, vtime: u64) -> Self {
        Self {
            vtime,
            lc: 0.0,
            prev_decay_ns: now_ns,
        }
    }

    fn discrete_decay(&mut self, now_ns: u64) {
        const HALF_LIFE_NS: u64 = 1_000_000;

        let delta_ns = now_ns.saturating_sub(self.prev_decay_ns);
        if delta_ns > 0 {
            let periods = delta_ns / HALF_LIFE_NS;
            if periods > 0 {
                self.lc = if periods >= 64 {
                    0.0
                } else {
                    (self.lc as u64 >> periods) as f64
                };
                self.prev_decay_ns = now_ns - (delta_ns % HALF_LIFE_NS);
            }
        }
    }

    fn continuous_decay(&mut self, now_ns: u64) {
        const HALF_LIFE_NS: f64 = 1_000_000.0;
        let delta_ns = now_ns.saturating_sub(self.prev_decay_ns);
        if delta_ns > 0 {
            let decay_factor = (0.5_f64).powf(delta_ns as f64 / HALF_LIFE_NS);
            self.lc *= decay_factor;
        }
        self.prev_decay_ns = now_ns;
    }
}

impl<'a> Scheduler<'a> {
    pub fn init(
        open_object: &'a mut MaybeUninit<OpenObject>,
        settings: &'a Settings,
    ) -> Result<Self> {
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
            system_vtime: 0,
            settings,
        })
    }

    fn consume_wake_events(&mut self) {
        while let Ok(Some(event)) = self.bpf.dequeue_wakeup_event() {
            let now = Self::now();

            let entry_waker = self
                .meta
                .entry(event.waker_pid)
                .or_insert(TaskMetadata::new(now, self.system_vtime));

            let waker_lc = {
                match self.settings.decay_function {
                    DecayFunction::Discrete => entry_waker.discrete_decay(now),
                    DecayFunction::Continous => entry_waker.continuous_decay(now),
                }
                entry_waker.lc
            };

            let entry_wakee = self
                .meta
                .entry(event.wakee_pid)
                .or_insert(TaskMetadata::new(now, self.system_vtime));

            match self.settings.decay_function {
                DecayFunction::Discrete => entry_wakee.discrete_decay(now),
                DecayFunction::Continous => entry_wakee.continuous_decay(now),
            }

            // If waker is blocked by a smaller priority task
            // we give some of our criticality so the less important task
            // doesn't block how much a critical task executes.
            //
            // We give / get just a portion so we don't elevate every task
            // to high criticality.
            if self.settings.inherit && waker_lc > 2.0 * entry_wakee.lc {
                let give = (waker_lc - (2.0 * entry_wakee.lc)) * 0.125;
                let can_get = entry_wakee.lc * 0.25;
                entry_wakee.lc += can_get.min(give);
            }

            entry_wakee.lc += LC_WAKEUP_BOOST;
        }
    }

    fn retrieve_new_tasks(&mut self) {
        while let Ok(Some(task)) = self.bpf.dequeue_task() {
            let now = Self::now();
            let weight = (task.weight as f64).max(1.0);
            let exec_runtime = task.exec_runtime as f64;

            let entry = self
                .meta
                .entry(task.pid)
                .or_insert(TaskMetadata::new(now, self.system_vtime));
            entry.vtime += (exec_runtime / weight) as u64;

            match self.settings.decay_function {
                DecayFunction::Discrete => entry.discrete_decay(now),
                DecayFunction::Continous => entry.continuous_decay(now),
            }

            self.system_vtime = self.system_vtime.max(entry.vtime);
            let v_deadline = entry.vtime + (RUNTIME_NS as f64 / (weight + entry.lc)) as u64;
            self.tasks.push(Task {
                inner: task,
                v_deadline,
            })
        }
    }

    fn drip_dispatch_tasks(&mut self) {
        let nr_waiting = self.tasks.len() as u64;
        if nr_waiting == 0 {
            self.bpf.notify_complete(0);
            return;
        }

        let exec_slice_ns = (RUNTIME_NS / (nr_waiting + 1)).max(MIN_SLICE_NS);
        if let Some(task) = self.tasks.pop() {
            let mut dispatched_task = DispatchedTask::new(&task.inner);
            let cpu = self.bpf.select_cpu(
                dispatched_task.pid,
                dispatched_task.cpu,
                dispatched_task.flags,
            );
            dispatched_task.cpu = if cpu >= 0 { cpu } else { RL_CPU_ANY };
            dispatched_task.slice_ns = exec_slice_ns;

            // TODO: dispatched_task.flags |= SCX_ENQ_PREEMPT;
            self.bpf.dispatch_task(&dispatched_task).unwrap();
        }
        self.bpf.notify_complete(self.tasks.len() as u64);
    }

    fn batch_dispatch_tasks(&mut self) {
        let nr_online_cpus = *self.bpf.nr_online_cpus_mut();
        let nr_queued_in_kernel = *self.bpf.nr_queued_mut();

        let nr_tasks_target = 2 * nr_online_cpus;
        if nr_queued_in_kernel >= nr_tasks_target {
            self.bpf.notify_complete(0);
            return;
        }

        let target_dispatches = nr_tasks_target - nr_queued_in_kernel;
        let mut dispatched = 0;

        let nr_waiting = self.tasks.len() as u64;
        let exec_slice_ns = (RUNTIME_NS / (nr_waiting + 1)).max(MIN_SLICE_NS);

        while let Some(task) = self.tasks.pop() {
            let mut dispatched_task = DispatchedTask::new(&task.inner);
            let cpu = self.bpf.select_cpu(
                dispatched_task.pid,
                dispatched_task.cpu,
                dispatched_task.flags,
            );
            dispatched_task.cpu = if cpu >= 0 { cpu } else { RL_CPU_ANY };
            dispatched_task.slice_ns = exec_slice_ns;

            self.bpf.dispatch_task(&dispatched_task).unwrap();
            dispatched += 1;

            if dispatched >= target_dispatches {
                break;
            }
        }
        self.bpf.notify_complete(self.tasks.len() as u64);
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

    pub fn run(&mut self) -> Result<UserExitInfo> {
        let mut prev_ts = Self::now();
        let stat_interval_ns = 1_000_000_000;

        while !self.bpf.exited() {
            self.consume_wake_events();
            self.retrieve_new_tasks();
            self.retrieve_new_tasks();

            let curr_ts = Self::now();
            if curr_ts.saturating_sub(prev_ts) > stat_interval_ns {
                self.print_stats();
                prev_ts = curr_ts;
            }

            match self.settings.dispatch_strategy {
                DispatchStrategy::DripFeed => self.drip_dispatch_tasks(),
                DispatchStrategy::Batch => self.batch_dispatch_tasks(),
            }
        }
        self.bpf.shutdown_and_report()
    }
}
