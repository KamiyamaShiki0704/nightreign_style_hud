//! Actual process-resident eleven-phase SDK registrations. No leaked forgotten task tickets.
use shikifsrs_elden_ring::{
    TaskCallback, TaskInvocation, TaskPhase, Ww171File, current_tasks::CurrentTaskEngine,
    task_resident::ResidentCallbackGroupSlot,
};
use shikifsrs_runtime::{Error, Result, memory::CurrentProcess};
use std::{
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    time::{Duration, Instant},
};
use windows::Win32::System::Threading::GetCurrentThreadId;
pub const PHASES: [&str; 11] = [
    "ChrIns_PreBehavior",
    "ChrIns_PreClothSafe",
    "DrawParamUpdate",
    "SoundStep",
    "ChrIns_PostPhysics",
    "ChrIns_PostPhysics",
    "DmgMan_Pre",
    "DmgMan_Post",
    "MenuMan",
    "ScaleformStep",
    "Draw_Pre",
];
static OWNERS: ResidentCallbackGroupSlot<11> = ResidentCallbackGroupSlot::new();
struct Admission {
    thread: AtomicU32,
    blocked: AtomicBool,
}
impl Admission {
    const fn new() -> Self {
        Self {
            thread: AtomicU32::new(0),
            blocked: AtomicBool::new(false),
        }
    }
    fn accept(&self, group: u32, expected: u32, thread: u32) -> bool {
        if self.blocked.load(Ordering::Acquire) {
            return false;
        }
        if group != expected || expected == 0 || thread == 0 {
            self.blocked.store(true, Ordering::Release);
            return false;
        }
        let matches =
            match self
                .thread
                .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => true,
                Err(v) => v == thread,
            };
        if !matches {
            self.blocked.store(true, Ordering::Release);
        }
        matches
    }
}
static ADMISSION: [Admission; 11] = [const { Admission::new() }; 11];
fn dispatch(index: usize, group: u32, callback: fn(), invocation: TaskInvocation) {
    // SAFETY: Thread id query does not call the game or acquire an engine lifetime.
    let thread = unsafe { GetCurrentThreadId() };
    if ADMISSION[index].accept(invocation.group_id, group, thread) {
        callback();
    } else {
        eprintln!("Shiki HUD phase {} admission rejected", PHASES[index]);
    }
}
/// Caller retains the pinned callbacks and experimental scheduler/allocator/graph contracts.
pub fn install(callbacks: [fn(); 11]) -> Result<()> {
    let (bytes, base) = crate::sdk_host::binding()?;
    let file = Ww171File::verify(&bytes)?;
    let mut phases = [TaskPhase::FRAME_BEGIN; 11];
    for (phase, name) in phases.iter_mut().zip(PHASES) {
        *phase = file.phase(name)?;
    }
    let start = Instant::now();
    let groups = loop {
        let catalog = file.bind(&CurrentProcess, base)?;
        let mut groups = [0; 11];
        for (index, phase) in phases.iter().copied().enumerate() {
            if let Some(id) = catalog
                .observe_phase(phase)?
                .and_then(|v| v.native_group_id())
                && catalog.registered_group(id, 4096)?.is_some()
            {
                groups[index] = id;
            }
        }
        if groups.iter().all(|v| *v != 0) {
            break groups;
        }
        if start.elapsed() >= Duration::from_secs(30) {
            return Err(Error::NativeOperationRejected("HUD task startup timeout"));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    // SAFETY: SDK and host DLL remain pinned; actual callbacks supply their native graph contracts.
    let engine = unsafe { CurrentTaskEngine::bind(bytes, base) }?;
    let tasks = std::array::from_fn(|index| {
        TaskCallback::new(move |invocation| {
            dispatch(index, groups[index], callbacks[index], invocation)
        })
    });
    // SAFETY: Static owner retains all successful and partial registrations for process lifetime.
    let installed = unsafe { OWNERS.install(engine, phases, tasks) }?;
    if !installed.registration?.registered_all() {
        return Err(Error::NativeOperationRejected(
            "partial HUD registration retained",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_admission_latches_mismatch_and_keeps_two_postphysics_callbacks() {
        assert_eq!(PHASES.len(), 11);
        assert_eq!(PHASES[4], PHASES[5]);
        let a = Admission::new();
        assert!(a.accept(7, 7, 12));
        assert!(a.accept(7, 7, 12));
        assert!(!a.accept(7, 7, 13));
        assert!(!a.accept(7, 7, 12));
        let b = Admission::new();
        assert!(!b.accept(8, 7, 12));
        assert!(!b.accept(7, 7, 12));
        let c = Admission::new();
        assert!(!c.accept(0, 0, 12));
    }
}
