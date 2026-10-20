//! HUD adapter for Summon's verified, deferred native destruction.
use eldenring::{
    cs::{CSNowLoadingHelper, CSTaskGroupIndex, ChrIns, FieldInsHandle, WorldChrMan},
    fd4::FD4TaskData,
};
use fromsoftware_shared::{FromStatic, program::Program};
use pelite::pe64::Pe;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleFileNameW};

/// Render holds this while using native characters. FrameEnd never waits on it.
pub(crate) static ACCESS: Mutex<()> = Mutex::new(());
pub(crate) static MODULE: AtomicUsize = AtomicUsize::new(0);
static LOG: Mutex<Option<File>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<Cleanup> = Mutex::new(Cleanup {
    session: None,
    staged: Vec::new(),
    unbound: Vec::new(),
});
static SUPPORTED: LazyLock<bool> = LazyLock::new(|| {
    let program = Program::current();
    let Ok(resources) = program.resources() else {
        return false;
    };
    let Ok(info) = resources.version_info() else {
        return false;
    };
    let Some(fixed) = info.fixed() else {
        return false;
    };
    let Some(language) = info.translation().first() else {
        return false;
    };
    let v = fixed.dwProductVersion;
    super::received_direction_version_supported(
        language.lang_id,
        [v.Major, v.Minor, v.Patch, v.Build],
    )
});

pub(crate) fn log(message: impl std::fmt::Display) {
    let Ok(mut file) = LOG.lock() else { return };
    if file.is_none() {
        let module = MODULE.load(Ordering::Relaxed);
        if module == 0 {
            return;
        }
        let mut path = [0u16; 32768];
        let len =
            unsafe { GetModuleFileNameW(Some(HMODULE(module as *mut _)), &mut path) } as usize;
        if len == 0 || len >= path.len() {
            return;
        }
        let path = std::path::PathBuf::from(String::from_utf16_lossy(&path[..len]));
        let Some(dir) = path.parent() else { return };
        *file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("nightreign_revenant_cleanup.log"))
            .ok();
    }
    if let Some(file) = file.as_mut() {
        let _ = writeln!(file, "[revenant] {message}");
        let _ = file.flush();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Session {
    world: usize,
    player: usize,
    load: i32,
}

struct Staged {
    handle: FieldInsHandle,
    character: usize,
    ready: Instant,
}
struct Cleanup {
    session: Option<Session>,
    staged: Vec<Staged>,
    unbound: Vec<super::RevenantDebugSoul>,
}

fn session_now() -> Option<Session> {
    let loading = unsafe { CSNowLoadingHelper::instance() }.ok()?;
    if super::loading_state::screen_active() != Some(false) {
        return None;
    }
    let world = unsafe { WorldChrMan::instance() }.ok()?;
    let player = super::retirement::player_address(world);
    (player != 0).then_some(Session {
        world: world as *const _ as usize,
        player,
        load: loading.menu_load_counter,
    })
}

fn session_changed(old: Option<Session>, current: Option<Session>) -> bool {
    old != current
}

/// Called under ACCESS from both render and FrameEnd, including while HUD hidden.
pub(crate) fn observe_session() -> bool {
    let current = session_now();
    let Ok(mut state) = STATE.lock() else {
        return false;
    };
    if session_changed(state.session, current) {
        if !state.staged.is_empty() {
            log(format_args!(
                "phase=quarantined staged={} reason=world-player-or-loading-changed; native-owned instances left to engine",
                state.staged.len()
            ));
        }
        state.staged.clear();
        state.unbound.clear();
        super::retirement::quarantine_all("world-player-or-loading-changed");
        state.session = current;
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    current.is_some()
}

pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

pub(crate) fn pending() -> bool {
    STATE
        .lock()
        .map_or(true, |s| !s.staged.is_empty() || !s.unbound.is_empty())
        || super::retirement::pending()
}

pub(crate) fn stage_unbound(soul: super::RevenantDebugSoul) {
    let Ok(mut state) = STATE.lock() else { return };
    if state.session.is_some()
        && !state
            .unbound
            .iter()
            .any(|s| s.requested_at == soul.requested_at)
    {
        state.unbound.push(soul);
        log("phase=awaiting-create-result reason=cleanup-before-bind");
    }
}

fn bind_cancelled_requests(world: &mut WorldChrMan) {
    if world.debug_chr_creator().spawn {
        return;
    }
    let requests = {
        let Ok(mut state) = STATE.lock() else { return };
        std::mem::take(&mut state.unbound)
    };
    for mut soul in requests {
        let created = world.debug_chr_creator().last_created_chr.and_then(|ptr| {
            let chr = unsafe { ptr.as_ref() };
            super::revenant_created_soul_matches(&soul, chr)
                .then_some((chr.field_ins_handle, chr as *const ChrIns as usize))
        });
        if let Some((handle, character)) = created {
            soul.handle = Some(handle);
            soul.character = character;
            super::release_revenant_debug_soul(world, &soul);
        } else {
            log(
                "phase=unbound-no-result reason=creator-consumed-without-owned-match; no unrelated unit touched",
            );
        }
    }
}

/// Caller has already made its owned soul inert; ownership survives role resets.
pub(crate) fn stage(world: &mut WorldChrMan, handle: FieldInsHandle, character: usize) {
    let Ok(mut state) = STATE.lock() else { return };
    let Some(session) = state.session else { return };
    if session.world != world as *mut _ as usize
        || session.player != super::retirement::player_address(world)
    {
        return;
    }
    if state
        .staged
        .iter()
        .any(|s| s.handle == handle && s.character == character)
    {
        return;
    }
    state.staged.push(Staged {
        handle,
        character,
        ready: Instant::now() + Duration::from_millis(1),
    });
    log(format_args!(
        "phase=staged handle={handle} character={character:#x}"
    ));
}

fn is_frame_end(group: u32) -> bool {
    group == CSTaskGroupIndex::FrameEnd as u32 || group == 0x9000_00a8
}

pub(crate) fn frame_end(data: &FD4TaskData) {
    if !is_frame_end(data.task_group_id) {
        return;
    }
    let Ok(_access) = ACCESS.try_lock() else {
        return;
    };
    if !observe_session() {
        return;
    }
    let Ok(world) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    if !*SUPPORTED {
        return;
    }
    bind_cancelled_requests(world);
    // Poll BEFORE new detachment, so this callback cannot count as a quiet frame.
    super::retirement::poll(world);
    let Ok(mut state) = STATE.lock() else { return };
    let now = Instant::now();
    let mut index = 0;
    while index < state.staged.len() {
        let item = &mut state.staged[index];
        if now < item.ready {
            index += 1;
            continue;
        }
        let current = world
            .chr_ins_by_handle(&item.handle)
            .map(|chr| chr as *const ChrIns as usize);
        if current != Some(item.character) {
            log(format_args!(
                "phase=abandoned handle={} reason=missing-or-replaced-instance",
                item.handle
            ));
            state.staged.swap_remove(index);
            continue;
        }
        let outcome = super::chr_lifecycle::destroy_summon(world, item.handle);
        match outcome {
            super::chr_lifecycle::DestroyOutcome::RetirementStarted
            | super::chr_lifecycle::DestroyOutcome::NotFound => {
                state.staged.swap_remove(index);
            }
            _ => {
                log(format_args!(
                    "phase=refused handle={} outcome={outcome}; retry=1s",
                    item.handle
                ));
                item.ready = now + Duration::from_secs(1);
                index += 1;
            }
        }
    }
}

pub(crate) fn installed() {
    log(format_args!(
        "init retirement=Summon-01491ed supported_ww271={} dispatch=168-or-0x900000a8",
        *SUPPORTED
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_verified_frame_end_encodings_are_accepted() {
        assert!(is_frame_end(168));
        assert!(is_frame_end(2_415_919_272));
        for id in [0, 167, 169, 0x8000_00a8, 0x9000_00a7, u32::MAX] {
            assert!(!is_frame_end(id));
        }
    }
    #[test]
    fn transitions_invalidate_even_reused_world_and_player_addresses() {
        let session = Session {
            world: 1,
            player: 2,
            load: 3,
        };
        assert!(!session_changed(Some(session), Some(session)));
        for next in [
            None,
            Some(Session {
                world: 4,
                ..session
            }),
            Some(Session {
                player: 5,
                ..session
            }),
            Some(Session { load: 4, ..session }),
        ] {
            assert!(session_changed(Some(session), next));
        }
    }
}
