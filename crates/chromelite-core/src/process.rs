use crate::tab::TabId;
use std::collections::{HashMap, HashSet};
use tracing::info;

pub type RendererProcessId = u32;

pub struct ProcessPool {
    max_processes: usize,
    active_processes: HashSet<RendererProcessId>,
    tab_allocation: HashMap<TabId, RendererProcessId>,
    next_pid: RendererProcessId,
}

impl ProcessPool {
    pub fn new(max_processes: usize) -> Self {
        Self {
            max_processes: max_processes.max(1),
            active_processes: HashSet::new(),
            tab_allocation: HashMap::new(),
            next_pid: 1000,
        }
    }

    pub fn assign_process_for_tab(&mut self, tab_id: TabId) -> RendererProcessId {
        if let Some(&pid) = self.tab_allocation.get(&tab_id) {
            return pid;
        }

        // Reuse least loaded process if at capacity
        if self.active_processes.len() >= self.max_processes {
            // Find existing process with fewest tabs
            let mut counts: HashMap<RendererProcessId, usize> = HashMap::new();
            for &pid in self.active_processes.iter() {
                counts.insert(pid, 0);
            }
            for &pid in self.tab_allocation.values() {
                *counts.entry(pid).or_insert(0) += 1;
            }

            let best_pid = *counts.iter().min_by_key(|(_, &count)| count).unwrap().0;
            self.tab_allocation.insert(tab_id, best_pid);
            info!(tab_id, pid = best_pid, "Assigned tab to shared renderer process");
            best_pid
        } else {
            let new_pid = self.next_pid;
            self.next_pid += 1;
            self.active_processes.insert(new_pid);
            self.tab_allocation.insert(tab_id, new_pid);
            info!(tab_id, pid = new_pid, "Spawned new bounded renderer process");
            new_pid
        }
    }

    pub fn release_tab(&mut self, tab_id: TabId) {
        if let Some(pid) = self.tab_allocation.remove(&tab_id) {
            let still_used = self.tab_allocation.values().any(|&p| p == pid);
            if !still_used {
                self.active_processes.remove(&pid);
                info!(pid, "Terminated idle renderer process");
            }
        }
    }
}

pub struct ProcessSupervisor {
    pub pool: ProcessPool,
}

impl ProcessSupervisor {
    pub fn new(max_processes: usize) -> Self {
        Self {
            pool: ProcessPool::new(max_processes),
        }
    }
}
