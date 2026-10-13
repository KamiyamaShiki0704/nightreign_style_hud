//! Original phase adapter. Native graph lifetime and frame ordering are experimental.
use crate::sdk_host;
use shikifsrs_elden_ring::{
    TaskCallback, TaskInvocation, Ww171File,
    camera_adjustments::{
        CameraScalar, CameraScalarKey, CameraScalarObservation, CameraScalarWrite, CameraSlot,
        CurrentCameraScalars,
    },
    cameras::{
        CameraOverrideObservation, CameraOverrideWrite, CurrentCameraOverrides,
        PerspectiveCameraObservation,
    },
    character_flags::CharacterFlagWrite,
    character_sets::CharacterIdentity,
    controller_policy::ControllerPolicyWrite,
    current_character_flags::CurrentCharacterFlags,
    current_controller_policy::CurrentControllerPolicy,
    current_tasks::CurrentTaskEngine,
    task_resident::ResidentCallbackGroupSlot,
};
use shikifsrs_runtime::{Error, Result, memory::CurrentProcess};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::System::Threading::GetCurrentThreadId;

#[derive(Clone, Copy)]
pub struct PrecisionSettings {
    pub fov_scale: f32,
    pub pullback: f32,
    pub right_offset: f32,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ActivationPolicy {
    pub enabled: bool,
    pub forced_aim: bool,
    pub standalone: bool,
    pub calibration: bool,
}
struct Configuration {
    settings: PrecisionSettings,
    policy: fn(&[i32]) -> ActivationPolicy,
}
static CONFIGURATION: OnceLock<Configuration> = OnceLock::new();
static MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub fn configure(
    settings: PrecisionSettings,
    policy: fn(&[i32]) -> ActivationPolicy,
) -> Result<()> {
    if !settings.fov_scale.is_finite()
        || settings.fov_scale <= 0.0
        || !settings.pullback.is_finite()
        || !settings.right_offset.is_finite()
    {
        return Err(Error::NativeOperationRejected(
            "invalid precision configuration",
        ));
    }
    CONFIGURATION
        .set(Configuration { settings, policy })
        .map_err(|_| Error::NativeOperationRejected("precision configuration already claimed"))
}
#[derive(Debug, Clone, Copy)]
pub struct PrecisionStatus {
    pub active: bool,
    pub standalone: bool,
    pub calibration: bool,
}
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub static ACTIVE: AtomicBool = AtomicBool::new(false);
static SUPPRESSED: AtomicBool = AtomicBool::new(false);
static HKS_SUPPRESSED: AtomicBool = AtomicBool::new(false);
static TASKS: ResidentCallbackGroupSlot<9> = ResidentCallbackGroupSlot::new();
static THREADS: [AtomicU32; 9] = [const { AtomicU32::new(0) }; 9];
static NORMAL_FOVS: Mutex<Option<[Option<CameraScalarObservation>; 8]>> = Mutex::new(None);
static NORMAL_GAME: Mutex<Option<CameraOverrideObservation>> = Mutex::new(None);
const SLOTS: [CameraSlot; 8] = [
    CameraSlot::Perspective0,
    CameraSlot::Perspective1,
    CameraSlot::Perspective2,
    CameraSlot::Perspective3,
    CameraSlot::Character,
    CameraSlot::Follow,
    CameraSlot::Aim,
    CameraSlot::Distance,
];
const PHASES: [&str; 9] = [
    "ChrIns_PreBehavior",
    "ChrIns_PreClothSafe",
    "DrawParamUpdate",
    "SoundStep",
    "ChrIns_PostPhysics",
    "DmgMan_Pre",
    "DmgMan_Post",
    "ScaleformStep",
    "Draw_Pre",
];

#[derive(Debug)]
enum Receipt {
    Scalar(CameraScalarWrite),
    Game(CameraOverrideWrite),
    Flag(CharacterFlagWrite),
    Policy(ControllerPolicyWrite),
}
impl Receipt {
    fn good(&self) -> bool {
        match self {
            Self::Scalar(v) => v.postflight.is_ok(),
            Self::Game(v) => v.postflight.is_ok(),
            Self::Flag(v) => v.postflight.is_ok(),
            Self::Policy(v) => v.postflight.is_ok(),
        }
    }
}
struct Session {
    epoch: u64,
    scalars: CurrentCameraScalars,
    game: CurrentCameraOverrides,
    flags: CurrentCharacterFlags,
    policy: CurrentControllerPolicy,
    receipts: VecDeque<Receipt>,
    failed: Option<Receipt>,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    quarantined: Vec<Session>,
}
thread_local! { static WRITERS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
impl Session {
    fn retain(&mut self, receipt: Receipt) -> Result<()> {
        if !receipt.good() {
            self.failed = Some(receipt);
            return Err(Error::NativeOperationRejected(
                "precision write postflight failed; retained",
            ));
        }
        self.receipts.push_back(receipt);
        if self.receipts.len() > 64 {
            self.receipts.pop_front();
        }
        Ok(())
    }
    fn request(&mut self, target: CharacterIdentity, value: bool) -> Result<()> {
        let source = self
            .flags
            .observe(target)?
            .ok_or(Error::NativeOperationRejected("precision player missing"))?;
        // SAFETY: Actual exclusive native callback graph/phase remains a host obligation.
        let receipt = unsafe { self.flags.set_precision_aim_requested(source, value) }?;
        self.retain(Receipt::Flag(receipt))
    }
    fn suppress(&mut self, target: CharacterIdentity, value: bool) -> Result<()> {
        let source = self
            .policy
            .observe(target)?
            .ok_or(Error::NativeOperationRejected(
                "precision controller missing",
            ))?;
        // SAFETY: Actual exclusive controller/modifier graph remains a host obligation.
        let receipt = unsafe { self.policy.set_precision_aim_suppressed(source, value) }?;
        self.retain(Receipt::Policy(receipt))
    }
    fn scalar(&mut self, source: CameraScalarObservation, bits: u32) -> Result<()> {
        // SAFETY: Actual phase retains writable selected word, owner path and no ABA.
        let receipt = unsafe { self.scalars.compare_replace(source, bits) }?;
        self.retain(Receipt::Scalar(receipt))
    }
    fn remember(&self) -> Result<()> {
        let mut fovs = [None; 8];
        for (index, slot) in SLOTS.into_iter().enumerate() {
            fovs[index] = self.scalars.observe(CameraScalarKey {
                epoch: self.epoch,
                slot,
                field: CameraScalar::FieldOfView,
            })?;
        }
        if fovs[..4].iter().all(Option::is_some)
            && fovs
                .iter()
                .flatten()
                .all(|v| v.value().is_finite() && v.value() > 0.05 && v.value() < 3.2)
        {
            *NORMAL_FOVS
                .lock()
                .map_err(|_| Error::NativeOperationRejected("FOV baseline lock"))? = Some(fovs);
        }
        if let Some(game) = self.game.observe()?
            && game
                .payload
                .scalar_bits
                .iter()
                .all(|bits| f32::from_bits(*bits).is_finite())
        {
            *NORMAL_GAME
                .lock()
                .map_err(|_| Error::NativeOperationRejected("game camera baseline lock"))? =
                Some(game);
        }
        Ok(())
    }
    fn restore_game(&mut self) -> Result<()> {
        let saved = *NORMAL_GAME
            .lock()
            .map_err(|_| Error::NativeOperationRejected("game camera baseline lock"))?;
        let Some(saved) = saved else { return Ok(()) };
        let Some(current) = self.game.observe()? else {
            return Ok(());
        };
        if saved.epoch != current.epoch
            || saved.module_base != current.module_base
            || saved.manager != current.manager
        {
            return Err(Error::NativeOperationRejected(
                "game camera baseline identity changed",
            ));
        }
        // SAFETY: Fresh comparison and original payload; caller excludes native timer updates.
        let receipt = unsafe { self.game.compare_replace(current, saved.payload) }?;
        self.retain(Receipt::Game(receipt))
    }
    fn restore_fovs(&mut self) -> Result<()> {
        let saved = *NORMAL_FOVS
            .lock()
            .map_err(|_| Error::NativeOperationRejected("FOV baseline lock"))?;
        let Some(saved) = saved else { return Ok(()) };
        for (index, baseline) in saved.into_iter().enumerate() {
            let Some(baseline) = baseline else { continue };
            if baseline.key.epoch != self.epoch {
                return Err(Error::NativeOperationRejected("FOV baseline epoch changed"));
            }
            let Some(current) = self.scalars.observe(baseline.key)? else {
                return Err(Error::NativeOperationRejected("FOV camera disappeared"));
            };
            if !same_camera(current, baseline) {
                return Err(Error::NativeOperationRejected("FOV baseline owner changed"));
            }
            let bits = if index < 4 {
                baseline.bits
            } else {
                let Some(follow) = saved[5] else { continue };
                follow.value().max(0.05).to_bits()
            };
            self.scalar(current, bits)?;
        }
        Ok(())
    }
    fn visual(&mut self) -> Result<()> {
        let settings = CONFIGURATION
            .get()
            .ok_or(Error::NativeOperationRejected("precision not configured"))?
            .settings;
        let Some(camera) = perspective()? else {
            return Ok(());
        };
        let Some(delta) = display_delta(camera) else {
            return Ok(());
        };
        for slot in SLOTS[..4].iter().copied() {
            for (index, field) in [
                CameraScalar::FieldOfView,
                CameraScalar::PositionX,
                CameraScalar::PositionY,
                CameraScalar::PositionZ,
            ]
            .into_iter()
            .enumerate()
            {
                let key = CameraScalarKey {
                    epoch: self.epoch,
                    slot,
                    field,
                };
                let Some(current) = self.scalars.observe(key)? else {
                    return Err(Error::NativeOperationRejected("visual camera missing"));
                };
                let value = if index == 0 {
                    (current.value() * settings.fov_scale).clamp(0.05, 3.0)
                } else {
                    current.value() - delta[index - 1]
                };
                self.scalar(current, value.to_bits())?;
            }
        }
        Ok(())
    }
}
fn same_camera(a: CameraScalarObservation, b: CameraScalarObservation) -> bool {
    a.key == b.key
        && a.module_base == b.module_base
        && a.owner == b.owner
        && a.parent == b.parent
        && a.camera == b.camera
}
fn with_session(run: impl FnOnce(&mut Session) -> Result<()>) -> Result<()> {
    WRITERS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive precision callback"))?;
        let epoch = sdk_host::epoch();
        if slot.current.as_ref().is_some_and(|v| v.epoch != epoch) {
            if slot.quarantined.len() >= 64 {
                return Err(Error::NativeOperationRejected(
                    "precision epoch quarantine bound",
                ));
            }
            let old = slot.current.take().unwrap();
            slot.quarantined.push(old);
            ACTIVE.store(false, Ordering::Relaxed);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Current callback supplies the experimental complete graph/phase contract.
            slot.current = Some(unsafe {
                Session {
                    epoch,
                    scalars: CurrentCameraScalars::bind(
                        bytes.clone(),
                        base,
                        epoch,
                        sdk_host::patches(),
                    )?,
                    game: CurrentCameraOverrides::bind(
                        bytes.clone(),
                        base,
                        epoch,
                        sdk_host::patches(),
                    )?,
                    flags: CurrentCharacterFlags::bind(
                        bytes.clone(),
                        base,
                        epoch,
                        sdk_host::patches(),
                    )?,
                    policy: CurrentControllerPolicy::bind(bytes, base, epoch, sdk_host::patches())?,
                    receipts: VecDeque::new(),
                    failed: None,
                }
            });
        }
        let session = slot.current.as_mut().unwrap();
        if session.failed.is_some() {
            return Err(Error::NativeOperationRejected(
                "unreconciled precision write",
            ));
        }
        run(session)
    })
}
#[derive(Clone, Copy)]
struct Player {
    target: CharacterIdentity,
    activation: bool,
    forced_aim: bool,
    manual: bool,
    requested: bool,
}
fn player() -> Result<Option<Player>> {
    sdk_host::with_objects(|objects, epoch| {
        let Some(target) = objects.primary_player_identity(epoch)? else {
            return Ok(None);
        };
        let effects = objects
            .character_effects(target, 4096)?
            .ok_or(Error::NativeOperationRejected("precision effects missing"))?;
        let input = objects
            .character_aim_input(target)?
            .ok_or(Error::NativeOperationRejected(
                "precision input module missing",
            ))?;
        let flags = objects
            .character_flags(target)?
            .ok_or(Error::NativeOperationRejected("precision flags missing"))?;
        let ids: Vec<i32> = effects.entries.iter().map(|v| v.native_id).collect();
        let config = CONFIGURATION
            .get()
            .ok_or(Error::NativeOperationRejected("precision not configured"))?;
        let policy = (config.policy)(&ids);
        MODE.store(
            u8::from(policy.standalone) | (u8::from(policy.calibration) << 1),
            Ordering::Relaxed,
        );
        Ok(Some(Player {
            target,
            activation: policy.enabled,
            forced_aim: policy.forced_aim,
            manual: input.manual_aim_input_admitted(),
            requested: flags.precision_aim_requested(),
        }))
    })
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Coordination {
    active: bool,
    suppressed: bool,
    hks_suppressed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Remember,
    RestoreGame,
    RestoreFovs,
    Visual,
    Request(bool),
    Suppress(bool),
}
fn plan(
    index: usize,
    player: Option<Player>,
    mut state: Coordination,
) -> Result<(Coordination, Vec<Action>)> {
    let mut actions = Vec::new();
    if index > 8 {
        return Err(Error::NativeOperationRejected("unknown precision phase"));
    }
    let finishing = index == 7 && state.suppressed;
    if index == 7 {
        state.suppressed = false;
    }
    let Some(player) = player else {
        if index == 0 {
            state.active = false;
            state.suppressed = false;
            actions.push(Action::Remember);
        }
        if index == 2 {
            state.active = false;
            actions.push(Action::Remember);
        }
        return Ok((state, actions));
    };
    if !player.activation {
        if index == 0 {
            state.active = false;
            state.suppressed = false;
            actions.extend([Action::Suppress(false), Action::Remember]);
        }
        if index == 2 {
            state.active = false;
            actions.push(Action::Remember);
        }
        if finishing {
            state.active = false;
        }
        return Ok((state, actions));
    }
    let aiming = (player.manual || player.forced_aim) || player.requested;
    match index {
        0 => {
            state.active = aiming;
            if !aiming {
                state.suppressed = false;
                state.hks_suppressed = false;
                actions.extend([Action::Suppress(false), Action::Remember]);
            } else {
                actions.push(Action::RestoreGame);
                if player.manual || player.forced_aim {
                    actions.push(Action::Request(true));
                }
                actions.push(Action::Suppress(false));
            }
        }
        1 => {
            state.active = aiming;
            if player.requested {
                state.suppressed = true;
                actions.push(Action::Request(false));
            }
        }
        2 => {
            state.active = aiming || state.suppressed;
            if !state.active {
                actions.push(Action::Remember);
            } else {
                actions.extend([
                    Action::RestoreGame,
                    Action::RestoreFovs,
                    Action::RestoreGame,
                ]);
                if (player.manual || player.forced_aim) || state.suppressed {
                    actions.push(Action::Request(true));
                }
                state.hks_suppressed = true;
                actions.push(Action::Suppress(true));
            }
        }
        3..=5 => {
            if (player.manual || player.forced_aim) || state.suppressed {
                state.active = true;
                actions.push(Action::Request(true));
            }
        }
        6 => {
            if aiming {
                state.active = true;
                state.suppressed = true;
                actions.push(Action::Request(false));
            }
        }
        7 => {
            if finishing {
                if state.hks_suppressed {
                    state.hks_suppressed = false;
                    actions.push(Action::Suppress(false));
                }
                if player.manual || player.forced_aim {
                    actions.push(Action::Request(true));
                }
            }
        }
        8 => {
            if state.active {
                actions.extend([Action::RestoreFovs, Action::RestoreGame, Action::Visual]);
            }
        }
        _ => unreachable!(),
    }
    Ok((state, actions))
}
pub fn advance(index: usize) -> Result<PrecisionStatus> {
    let observed = player()?;
    let state = Coordination {
        active: ACTIVE.load(Ordering::Relaxed),
        suppressed: SUPPRESSED.load(Ordering::Relaxed),
        hks_suppressed: HKS_SUPPRESSED.load(Ordering::Relaxed),
    };
    let (state, actions) = plan(index, observed, state)?;
    ACTIVE.store(state.active, Ordering::Relaxed);
    SUPPRESSED.store(state.suppressed, Ordering::Relaxed);
    HKS_SUPPRESSED.store(state.hks_suppressed, Ordering::Relaxed);
    with_session(|session| {
        for action in actions {
            match action {
                Action::Remember => session.remember()?,
                Action::RestoreGame => session.restore_game()?,
                Action::RestoreFovs => session.restore_fovs()?,
                Action::Visual => session.visual()?,
                Action::Request(value) => session.request(
                    observed.expect("plan requires player for request").target,
                    value,
                )?,
                Action::Suppress(value) => session.suppress(
                    observed
                        .expect("plan requires player for suppression")
                        .target,
                    value,
                )?,
            }
        }
        Ok(())
    })?;
    let mode = MODE.load(Ordering::Relaxed);
    Ok(PrecisionStatus {
        active: state.active,
        standalone: mode & 1 != 0,
        calibration: mode & 2 != 0,
    })
}
fn callback(index: usize, expected_group: u32, invocation: TaskInvocation) {
    // SAFETY: Windows thread ID query only; this does not grant a phase lock.
    let thread = unsafe { GetCurrentThreadId() };
    let matches =
        match THREADS[index].compare_exchange(0, thread, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => true,
            Err(old) => old == thread,
        };
    if !matches || invocation.group_id != expected_group {
        ACTIVE.store(false, Ordering::Relaxed);
        return;
    }
    match advance(index) {
        Ok(status) => tracing::trace!(
            phase = PHASES[index],
            active = status.active,
            standalone = status.standalone,
            calibration = status.calibration,
            "precision phase status"
        ),
        Err(error) => {
            ACTIVE.store(false, Ordering::Relaxed);
            tracing::error!(phase = PHASES[index], ?error, "precision callback rejected");
        }
    }
}
#[allow(dead_code)] // Standalone owns nine registrations; HUD calls advance through its eleven owners.
pub fn install() -> Result<()> {
    let (bytes, base) = sdk_host::binding()?;
    let file = Ww171File::verify(&bytes)?;
    let mut phases = [shikifsrs_elden_ring::TaskPhase::FRAME_BEGIN; 9];
    for (phase, name) in phases.iter_mut().zip(PHASES) {
        *phase = file.phase(name)?;
    }
    let start = Instant::now();
    let groups = loop {
        let catalog = file.bind(&CurrentProcess, base)?;
        let mut groups = [0; 9];
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
        if start.elapsed() > Duration::from_secs(30) {
            return Err(Error::NativeOperationRejected(
                "precision task startup timeout",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    // SAFETY: Host DLL is pinned; actual scheduler/allocator and callback graphs remain experimental.
    let engine = unsafe { CurrentTaskEngine::bind(bytes, base) }?;
    let callbacks = std::array::from_fn(|index| {
        TaskCallback::new(move |invocation| callback(index, groups[index], invocation))
    });
    // SAFETY: Static slot retains every partial registration outcome through process lifetime.
    let installed = unsafe { TASKS.install(engine, phases, callbacks) }?;
    if !installed.registration?.registered_all() {
        return Err(Error::NativeOperationRejected(
            "partial precision registration retained",
        ));
    }
    Ok(())
}
pub fn perspective() -> Result<Option<PerspectiveCameraObservation>> {
    sdk_host::with_objects(|objects, _| {
        let Some(cameras) = objects.cameras()? else {
            return Ok(None);
        };
        let Some(camera) = cameras.perspectives[0] else {
            return Ok(None);
        };
        objects.perspective_camera(camera.address)
    })
}
pub fn display_delta(camera: PerspectiveCameraObservation) -> Option<[f32; 3]> {
    let settings = CONFIGURATION.get()?.settings;
    let forward: [f32; 3] =
        std::array::from_fn(|index| f32::from_bits(camera.matrix_bits[2][index]));
    let right: [f32; 3] = std::array::from_fn(|index| f32::from_bits(camera.matrix_bits[0][index]));
    let len = dot3(forward, forward).sqrt();
    let right_len = dot3(right, right).sqrt();
    if !len.is_finite() || len < 0.001 || !right_len.is_finite() || right_len < 0.001 {
        return None;
    }
    Some(std::array::from_fn(|index| {
        forward[index] / len * settings.pullback + right[index] / right_len * settings.right_offset
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn player(manual: bool, requested: bool, activation: bool) -> Player {
        Player {
            target: CharacterIdentity {
                address: 8,
                world_address: 16,
                native_identifier: 32,
                epoch: 1,
            },
            activation,
            forced_aim: false,
            manual,
            requested,
        }
    }
    #[test]
    fn forced_calibration_aim_keeps_projectile_and_menu_phases_without_manual_native_bit() {
        let mut p = player(false, false, true);
        p.forced_aim = true;
        let (state, actions) = plan(0, Some(p), Coordination::default()).unwrap();
        assert!(state.active);
        assert!(actions.contains(&Action::Request(true)));
        let (_, actions) = plan(4, Some(p), state).unwrap();
        assert!(actions.contains(&Action::Request(true)));
        let (state, actions) = plan(6, Some(p), state).unwrap();
        assert!(state.suppressed);
        assert!(actions.contains(&Action::Request(false)));
        let (_, actions) = plan(7, Some(p), state).unwrap();
        assert!(actions.contains(&Action::Request(true)));
    }
    #[test]
    fn manual_aim_keeps_projectile_windows_and_restores_after_menu() {
        let mut state = Coordination::default();
        let mut requested = false;
        let mut suppression = false;
        for index in 0..9 {
            let (next, actions) = plan(index, Some(player(true, requested, true)), state).unwrap();
            state = next;
            for action in &actions {
                if let Action::Request(value) = *action {
                    requested = value;
                }
                if let Action::Suppress(value) = *action {
                    suppression = value;
                }
            }
            if index == 1 || index == 6 {
                assert!(!requested);
                assert!(state.suppressed);
            }
            if (2..=5).contains(&index) {
                assert!(requested);
                assert!(suppression);
            }
            if index == 7 {
                assert!(requested);
                assert!(!suppression);
                assert!(!state.suppressed);
            }
            if index == 8 {
                assert_eq!(
                    actions,
                    [Action::RestoreFovs, Action::RestoreGame, Action::Visual]
                );
            }
        }
        assert!(state.active);
    }
    #[test]
    fn inactive_effect_and_missing_player_never_schedule_visual_or_request_writes() {
        let state = Coordination {
            active: true,
            suppressed: true,
            hks_suppressed: true,
        };
        for observed in [None, Some(player(true, true, false))] {
            for index in 0..9 {
                let (_, actions) = plan(index, observed, state).unwrap();
                assert!(
                    actions
                        .iter()
                        .all(|v| matches!(v, Action::Remember | Action::Suppress(false)))
                );
            }
            assert!(!plan(7, observed, state).unwrap().0.suppressed);
        }
        assert!(plan(9, None, state).is_err());
    }
    #[test]
    fn native_request_without_manual_input_preserves_original_finish_behavior() {
        let state = Coordination {
            active: true,
            suppressed: true,
            hks_suppressed: true,
        };
        let (next, actions) = plan(7, Some(player(false, false, true)), state).unwrap();
        assert_eq!(actions, [Action::Suppress(false)]);
        assert!(next.active);
        assert!(!next.suppressed && !next.hks_suppressed);
        assert_eq!(
            plan(3, Some(player(false, false, true)), state).unwrap().1,
            [Action::Request(true)]
        );
    }
}
