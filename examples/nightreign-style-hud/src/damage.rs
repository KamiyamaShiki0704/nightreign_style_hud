static INSTALL_DAMAGE_HOOK: Once = Once::new();
static INSTALL_UNDERTAKER_AUTO_DODGE_HOOK: Once = Once::new();
static INSTALL_REVENANT_SUMMON_RANGE_HOOK: Once = Once::new();
static INSTALL_CAMERA_DIRECTED_POSITION_HOOK: Once = Once::new();
static INSTALL_MULTI_LOCK_BULLET_HOOK: Once = Once::new();
static MULTI_LOCK_BULLET_TARGET_CURSOR: AtomicUsize = AtomicUsize::new(0);
static MULTI_LOCK_BULLET_SPAWNING_CLONE: AtomicBool = AtomicBool::new(false);
static MULTI_LOCK_LIVE_BULLET_SEEN: LazyLock<Mutex<HashSet<native_bullets::Key>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static MULTI_LOCK_LIVE_BULLET_COUNT: AtomicU32 = AtomicU32::new(0);
static MULTI_LOCK_LIVE_GUIDED_BULLET_COUNT: AtomicU32 = AtomicU32::new(0);
static MULTI_LOCK_LIVE_WEAPON_GUIDANCE: LazyLock<
    Mutex<HashMap<native_bullets::Key, MultiLockLiveWeaponGuidance>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));
static MULTI_LOCK_SPAWN_HOOK_BULLET_PARAMS: LazyLock<Mutex<HashMap<i32, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static MULTI_LOCK_TARGET_MARKERS: LazyLock<Mutex<HashMap<HudHandle, i32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ULTIMATE_DAMAGE_EVENTS: AtomicU32 = AtomicU32::new(0);
static LOCAL_DAMAGE_MODULE: AtomicUsize = AtomicUsize::new(0);
static UNDERTAKER_AUTO_DODGE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static UNDERTAKER_AUTO_DODGE_ARMED: AtomicBool = AtomicBool::new(false);
static UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static UNDERTAKER_AUTO_DODGE_PENDING_HIT: AtomicU64 = AtomicU64::new(0);
static UNDERTAKER_AUTO_DODGE_DISPATCH_BLOCKED: AtomicBool = AtomicBool::new(false);
static UNDERTAKER_AUTO_DODGE_SUPPRESS_UNTIL_MS: AtomicUsize = AtomicUsize::new(0);
static UNDERTAKER_AUTO_DODGE_REARM_AT_MS: AtomicUsize = AtomicUsize::new(0);
static UNDERTAKER_AUTO_DODGE_CONTACTS: AtomicU32 = AtomicU32::new(0);
static UNDERTAKER_AUTO_DODGE_PULSES: AtomicU32 = AtomicU32::new(0);
static UNDERTAKER_AUTO_DODGE_EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);
static UNDERTAKER_HURTBOX_SENSOR: LazyLock<Mutex<UndertakerHurtboxSensor>> =
    LazyLock::new(|| Mutex::new(UndertakerHurtboxSensor::default()));
static REVENANT_SUMMON_RANGE_ACTIVE: AtomicBool = AtomicBool::new(false);
static DAMAGE_HOOK_CALLS: AtomicU32 = AtomicU32::new(0);
static DAMAGE_HOOK_POSITIVE: AtomicU32 = AtomicU32::new(0);
static DAMAGE_HOOK_EXCLUDED_LOCAL: AtomicU32 = AtomicU32::new(0);
static DAMAGE_HOOK_LAST_TARGET: AtomicUsize = AtomicUsize::new(0);
static DAMAGE_HOOK_LAST_DATA: AtomicUsize = AtomicUsize::new(0);
static DAMAGE_HOOK_LAST_OFFSET: AtomicU32 = AtomicU32::new(0);
static DAMAGE_HOOK_LAST_VALUE: AtomicU32 = AtomicU32::new(0);
static RECLUSE_DAMAGE_ELEMENTS: LazyLock<Mutex<HashMap<usize, RecluseElement>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static DAMAGE_HOOK_EVENTS: LazyLock<Mutex<Vec<DamageHookEvent>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static DUCHESS_REPLAY_HISTORY: LazyLock<Mutex<Vec<DuchessReplayEvent>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static DUCHESS_REPLAY_IGNORES: LazyLock<Mutex<HashMap<usize, DuchessReplayPayload>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CAMERA_DIRECTED_VERTICAL_BIAS: LazyLock<Mutex<f32>> = LazyLock::new(|| Mutex::new(0.0));

const UNDERTAKER_HURTBOX_RADIUS_MARGIN: f32 = 0.40;
const UNDERTAKER_HURTBOX_RADIUS_MIN: f32 = 0.05;
const UNDERTAKER_HURTBOX_RADIUS_MAX: f32 = 3.0;
const UNDERTAKER_HURTBOX_RADIUS_INFLATED_MAX: f32 = 4.0;
const UNDERTAKER_AUTO_DODGE_SUPPRESS_MS: usize = 650;
const UNDERTAKER_AUTO_DODGE_REARM_MS: usize = 900;
// ER 2.7 dispatcher 0x449D30 passes these fields to Atk resolver 0xD24760.
const UNDERTAKER_DAMAGE_ATK_ID_OFFSET: usize = 0x40;
const UNDERTAKER_DAMAGE_ATK_TABLE_OFFSET: usize = 0x44;
const UNDERTAKER_PENDING_UNKNOWN_ATTACK: u64 = 1;
const REVENANT_SUMMON_RANGE_BUDDY_TYPE_ID: u32 = 0x7D0;
const REVENANT_SUMMON_RANGE_OVERRIDE: f32 = 1000.0;
const CAMERA_DIRECTED_POSITION_OFFSET: usize = 0x70;
const CAMERA_DIRECTED_VERTICAL_SCALE: f32 = 1.0;
const CAMERA_DIRECTED_VERTICAL_BIAS_MIN: f32 = -0.045;
const CAMERA_DIRECTED_VERTICAL_BIAS_MAX: f32 = 0.16;
const CAMERA_DIRECTED_VERTICAL_BIAS_DECAY: f32 = 0.70;
const CAMERA_DIRECTED_VERTICAL_BIAS_RESPONSE: f32 = 0.28;
const CAMERA_DIRECTED_PITCH_DEADZONE: f32 = 0.08;
const CAMERA_DIRECTED_HORIZONTAL_MIX_MAX: f32 = 0.82;
const CAMERA_DIRECTED_HORIZONTAL_MIX_FULL_PITCH: f32 = 0.60;
const MULTI_LOCK_PLAYER_EFFECT_START: i32 = 882000;
const MULTI_LOCK_PLAYER_EFFECT_END: i32 = 882100;
const MULTI_LOCK_PLAYER_EFFECT_GROUP_SIZE: i32 = 5;
const MULTI_LOCK_TARGET_MARKER_START: i32 = 882100;
const MULTI_LOCK_BULLET_MAX_TARGETS: usize = 32;
const MULTI_LOCK_BULLET_TARGET_TIERS: [usize; 4] = [4, 8, 16, 32];
const MULTI_LOCK_BULLET_SCAN_MAX_ENTRIES: usize = 128;
const MULTI_LOCK_BULLET_EXTRA_CLONES_PER_SPAWN: usize = MULTI_LOCK_BULLET_MAX_TARGETS - 1;
const MULTI_LOCK_LIVE_WEAPON_GUIDANCE_SECONDS: f32 = 0.25;
const MULTI_LOCK_LIVE_WEAPON_GUIDANCE_MIX: f32 = 0.4;
const MULTI_LOCK_LIVE_WEAPON_ASSIGN_WINDOW_SECONDS: f32 = 0.18;
const MULTI_LOCK_LIVE_ALWAYS_GUIDE_PARAM_MIN: i32 = 40_000_000;
const MULTI_LOCK_SPAWN_HOOK_PARAM_SUPPRESS_SECONDS: f32 = 0.75;
const MULTI_LOCK_SCREEN_LENS_RADIUS_SCALE: f32 = 1.18;
const DAMAGE_VALUE_OFFSETS: [usize; 5] = [0x230, 0x234, 0x238, 0x23C, 0x240];
const DAMAGE_SCAN_START: usize = 0x200;
const DAMAGE_SCAN_END: usize = 0x280;
const DAMAGE_LOG_INTERVAL: f32 = 1.0;
const DUCHESS_REPLAY_HISTORY_SECONDS: f32 = 3.5;
const DUCHESS_REPLAY_HISTORY_CAP: usize = 512;
const RECLUSE_LOCK_DEBUG_POINTS: bool = false;
const RECLUSE_MARKER_HIT_HEIGHT_FACTOR: f32 = 0.55;
const RECLUSE_MARKER_AABB_HEIGHT_FACTOR: f32 = 0.58;
const RECLUSE_MARKER_FALLBACK_BODY_LIFT: f32 = 1.35;
const RECLUSE_MARKER_MIN_LOCK_LIFT: f32 = 0.35;
const RECLUSE_MARKER_MAX_LOCK_LIFT: f32 = 18.0;
const RESURRECTION_TEAMMATE_FALLBACK_HEAD_LIFT: f32 = 1.9;
const RESURRECTION_TEAMMATE_HEAD_PADDING: f32 = 0.35;
const RESURRECTION_TEAMMATE_MAX_HEAD_LIFT: f32 = 8.0;
type LockDebugPoints = [Option<[f32; 2]>; 5];

#[derive(Default)]
struct UndertakerHurtboxSensor {
    physics_module: usize,
    identity: Option<shikifsrs_elden_ring::character_sets::CharacterIdentity>,
    baseline_hit_radius: Option<f32>,
    inflated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UndertakerHitDispatchAction {
    Forward,
    Reject,
    Trigger,
}

#[derive(Clone, Copy)]
struct MultiLockLiveWeaponGuidance {
    target: HudHandle,
    created_at: Instant,
    expires_at: Instant,
}

#[derive(Clone, Copy)]
struct DamageHookEvent {
    target_damage_module: usize,
    damage: i32,
    created_at: Instant,
}

#[derive(Clone, Copy, Default)]
struct DuchessReplayPayload {
    hp_damage: i32,
    status: [i32; 7],
    super_armor_damage: f32,
}

impl DuchessReplayPayload {
    fn scaled(self, rate: f32) -> Self {
        Self {
            hp_damage: scale_i32(self.hp_damage, rate),
            status: self.status.map(|value| scale_i32(value, rate)),
            super_armor_damage: (self.super_armor_damage.max(0.0) * rate).floor(),
        }
    }

    fn add(&mut self, other: Self) {
        self.hp_damage = self.hp_damage.saturating_add(other.hp_damage.max(0));
        for (slot, value) in self.status.iter_mut().zip(other.status) {
            *slot = slot.saturating_add(value.max(0));
        }
        self.super_armor_damage += other.super_armor_damage.max(0.0);
    }

    fn has_effect(self) -> bool {
        self.hp_damage > 0
            || self.status.iter().any(|value| *value > 0)
            || self.super_armor_damage > 0.0
    }
}

#[derive(Clone, Copy)]
struct DuchessReplayEvent {
    target_damage_module: usize,
    payload: DuchessReplayPayload,
    created_at: Instant,
}

fn install_damage_hook_once() {
    INSTALL_DAMAGE_HOOK.call_once(|| {
        let hook_site = match native_hooks::address(0) {
            Ok(address) => address,
            Err(error) => {
                append_damage_log(&format!("[init] fixed Shiki hook site rejected: {error}"));
                return;
            }
        };
        let hook = unsafe {
            native_hooks::install_back(0, |regs: *mut ilhook::x64::Registers| {
                let target_damage_module = (*regs).rcx as usize;
                let damage_data = (*regs).r8 as usize;
                DAMAGE_HOOK_CALLS.fetch_add(1, Ordering::Relaxed);
                DAMAGE_HOOK_LAST_TARGET.store(target_damage_module, Ordering::Relaxed);
                DAMAGE_HOOK_LAST_DATA.store(damage_data, Ordering::Relaxed);
                if let Some(element) = find_recluse_damage_element(damage_data) {
                    record_recluse_damage_element(target_damage_module, element);
                }
                if let Some((_, damage)) = find_damage_candidate(damage_data) {
                    record_damage_hook_event(target_damage_module, damage);
                }
                if is_chargeable_damage_event(target_damage_module, damage_data) {
                    ULTIMATE_DAMAGE_EVENTS.fetch_add(1, Ordering::Relaxed);
                }
            })
        };

        match hook {
            Ok(()) => {
                append_damage_log(&format!("[init] damage hook installed at 0x{hook_site:X}"));
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] damage hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

fn install_undertaker_auto_dodge_hook_once() {
    INSTALL_UNDERTAKER_AUTO_DODGE_HOOK.call_once(|| {
        let hook_site = match native_hooks::address(1) {
            Ok(address) => address,
            Err(error) => {
                append_damage_log(&format!("[init] fixed Shiki hook site rejected: {error}"));
                return;
            }
        };
        let hook = unsafe {
            native_hooks::install_ret(1, |regs: *mut ilhook::x64::Registers, original: usize| {
                undertaker_hit_dispatch_destination(regs, original)
            })
        };

        match hook {
            Ok(()) => {
                UNDERTAKER_AUTO_DODGE_AVAILABLE.store(true, Ordering::Release);
                append_damage_log(&format!(
                    "[init] Undertaker auto-dodge hook installed at 0x{hook_site:X}"
                ));
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] Undertaker auto-dodge hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

unsafe extern "system" fn reject_undertaker_hit_dispatch(
    _target: usize,
    _source_chr: usize,
    _damage_data: usize,
    _flag: usize,
) -> usize {
    0
}

fn undertaker_hit_dispatch_destination(
    regs: *mut ilhook::x64::Registers,
    original: usize,
) -> usize {
    let regs = unsafe { &*regs };
    let target_damage_module = regs.rcx as usize;
    let damage_data = regs.r8 as usize;
    let local_damage_module = LOCAL_DAMAGE_MODULE.load(Ordering::Acquire);
    let now_ms = undertaker_auto_dodge_now_ms();
    let positive_damage = damage_data != 0 && find_damage_candidate(damage_data).is_some();
    let action = undertaker_hit_dispatch_action(
        target_damage_module,
        local_damage_module,
        positive_damage,
        UNDERTAKER_AUTO_DODGE_ARMED.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_PENDING_HIT.load(Ordering::Acquire) != 0,
        now_ms,
        UNDERTAKER_AUTO_DODGE_SUPPRESS_UNTIL_MS.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_REARM_AT_MS.load(Ordering::Acquire),
    );

    match action {
        UndertakerHitDispatchAction::Forward => original,
        UndertakerHitDispatchAction::Reject => reject_undertaker_hit_dispatch as *const () as usize,
        UndertakerHitDispatchAction::Trigger => {
            let pending_hit = capture_undertaker_pending_attack(damage_data);
            if UNDERTAKER_AUTO_DODGE_PENDING_HIT
                .compare_exchange(0, pending_hit, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                UNDERTAKER_AUTO_DODGE_SUPPRESS_UNTIL_MS.store(
                    now_ms.saturating_add(UNDERTAKER_AUTO_DODGE_SUPPRESS_MS),
                    Ordering::Release,
                );
                UNDERTAKER_AUTO_DODGE_REARM_AT_MS.store(
                    now_ms.saturating_add(UNDERTAKER_AUTO_DODGE_REARM_MS),
                    Ordering::Release,
                );
                UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
                UNDERTAKER_AUTO_DODGE_CONTACTS.fetch_add(1, Ordering::Relaxed);
            }
            reject_undertaker_hit_dispatch as *const () as usize
        }
    }
}

#[allow(clippy::too_many_arguments, reason = "Preserve the independently tested hit-admission policy inputs")]
fn undertaker_hit_dispatch_action(
    target_damage_module: usize,
    local_damage_module: usize,
    positive_damage: bool,
    armed: bool,
    sensor_active: bool,
    pending: bool,
    now_ms: usize,
    suppress_until_ms: usize,
    rearm_at_ms: usize,
) -> UndertakerHitDispatchAction {
    if target_damage_module == 0
        || local_damage_module == 0
        || target_damage_module != local_damage_module
    {
        return UndertakerHitDispatchAction::Forward;
    }
    if pending || now_ms < suppress_until_ms {
        return UndertakerHitDispatchAction::Reject;
    }
    if armed && sensor_active && now_ms >= rearm_at_ms && positive_damage {
        return UndertakerHitDispatchAction::Trigger;
    }
    UndertakerHitDispatchAction::Forward
}

fn undertaker_auto_dodge_now_ms() -> usize {
    UNDERTAKER_AUTO_DODGE_EPOCH
        .elapsed()
        .as_millis()
        .min(usize::MAX as u128) as usize
}

fn set_undertaker_auto_dodge_armed(armed: bool) {
    let armed = armed && UNDERTAKER_AUTO_DODGE_AVAILABLE.load(Ordering::Acquire);
    UNDERTAKER_AUTO_DODGE_ARMED.store(armed, Ordering::Release);
    if !armed {
        clear_undertaker_auto_dodge_latches();
    }
}

fn clear_undertaker_auto_dodge_latches() {
    UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
    if !UNDERTAKER_AUTO_DODGE_DISPATCH_BLOCKED.load(Ordering::Acquire) {
        UNDERTAKER_AUTO_DODGE_PENDING_HIT.store(0, Ordering::Release);
    }
    UNDERTAKER_AUTO_DODGE_SUPPRESS_UNTIL_MS.store(0, Ordering::Release);
    UNDERTAKER_AUTO_DODGE_REARM_AT_MS.store(0, Ordering::Release);
}

fn update_undertaker_auto_dodge_before_behavior() {
    let player = match native_world::primary_selection() {
        Ok(Some(selection)) => selection.player,
        Ok(None) => {
            set_undertaker_auto_dodge_armed(false);
            clear_undertaker_hurtbox_sensor_without_restore();
            return;
        }
        Err(error) => {
            eprintln!("ShikiFSRS HUD dodge primary selection rejected: {error}");
            return;
        }
    };
    let effects = match native_character::effects(player.address) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD dodge effects rejected: {error}");
            return;
        }
    };
    if UNDERTAKER_AUTO_DODGE_ARMED.load(Ordering::Acquire)
        && !effects.contains(&SP_UNDERTAKER_ENHANCED_SKILL_BUFF)
    {
        set_undertaker_auto_dodge_armed(false);
        native_character::report_effect_result(clear_undertaker_dodge_effects(player));
    }
    update_undertaker_hurtbox_sensor(player);
    apply_pending_undertaker_auto_dodge(player);
}

fn update_undertaker_hurtbox_sensor(
    player: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) {
    let source = match native_character::scalar(
        player.address,
        shikifsrs_elden_ring::character_scalars::CharacterScalar::CharacterHitRadius,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD hurtbox radius copy rejected: {error}");
            UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
            return;
        }
    };
    if source.key.character != player {
        eprintln!("ShikiFSRS HUD dodge radius full identity changed");
        UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
        return;
    }
    let radius = f32::from_bits(source.bits);
    let physics_module = source.storage.address as usize;
    let Ok(mut sensor) = UNDERTAKER_HURTBOX_SENSOR.lock() else {
        UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
        return;
    };

    if sensor.physics_module != physics_module || sensor.identity != Some(source.key.character) {
        *sensor = UndertakerHurtboxSensor {
            physics_module,
            identity: Some(source.key.character),
            baseline_hit_radius: valid_undertaker_hit_radius(radius).then_some(radius),
            inflated: false,
        };
    }

    let armed = UNDERTAKER_AUTO_DODGE_ARMED.load(Ordering::Acquire);
    let now_ms = undertaker_auto_dodge_now_ms();
    let cooling_down = UNDERTAKER_AUTO_DODGE_PENDING_HIT.load(Ordering::Acquire) != 0
        || now_ms < UNDERTAKER_AUTO_DODGE_REARM_AT_MS.load(Ordering::Acquire);
    let should_inflate = armed && !cooling_down;

    if should_inflate {
        if sensor.baseline_hit_radius.is_none() && valid_undertaker_hit_radius(radius) {
            sensor.baseline_hit_radius = Some(radius);
        }
        if let Some(baseline) = sensor.baseline_hit_radius {
            // SAFETY: Native callback retains the exact original scalar/module graph and
            // exclusive initialized writable phase. The SDK compares this copied source again.
            if let Err(error) = unsafe {
                native_character::replace_scalar(source, undertaker_inflated_hit_radius(baseline))
            } {
                eprintln!(
                    "ShikiFSRS HUD radius inflation failed or uncertain; receipt retained: {error}"
                );
                UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
                return;
            }
            sensor.inflated = true;
            UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(true, Ordering::Release);
            return;
        }
    }

    if sensor.inflated {
        if let Some(baseline) = sensor.baseline_hit_radius {
            // Refuse to overwrite a later owner's different value during restoration.
            if source.bits != undertaker_inflated_hit_radius(baseline).to_bits() {
                eprintln!("ShikiFSRS HUD radius restore conflict; baseline retained");
                UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
                return;
            }
            // SAFETY: Same exact scalar identity and exclusive graph contract; no Drop cleanup.
            if let Err(error) = unsafe { native_character::replace_scalar(source, baseline) } {
                eprintln!(
                    "ShikiFSRS HUD radius restore failed or uncertain; receipt retained: {error}"
                );
                UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
                return;
            }
        }
        sensor.inflated = false;
    }
    UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
    if !armed {
        *sensor = UndertakerHurtboxSensor::default();
    }
}

fn clear_undertaker_hurtbox_sensor_without_restore() {
    if let Ok(mut sensor) = UNDERTAKER_HURTBOX_SENSOR.lock() {
        *sensor = UndertakerHurtboxSensor::default();
    }
    UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.store(false, Ordering::Release);
}

fn valid_undertaker_hit_radius(radius: f32) -> bool {
    radius.is_finite()
        && (UNDERTAKER_HURTBOX_RADIUS_MIN..=UNDERTAKER_HURTBOX_RADIUS_MAX).contains(&radius)
}

fn undertaker_inflated_hit_radius(radius: f32) -> f32 {
    (radius + UNDERTAKER_HURTBOX_RADIUS_MARGIN).min(UNDERTAKER_HURTBOX_RADIUS_INFLATED_MAX)
}

fn capture_undertaker_pending_attack(damage_data: usize) -> u64 {
    if damage_data == 0 {
        return UNDERTAKER_PENDING_UNKNOWN_ATTACK;
    }
    // The hook owns this payload only for the duration of this native call.
    let (row_id, table) = unsafe {
        (
            read_i32(damage_data + UNDERTAKER_DAMAGE_ATK_ID_OFFSET),
            read_i32(damage_data + UNDERTAKER_DAMAGE_ATK_TABLE_OFFSET),
        )
    };
    pack_undertaker_pending_attack(row_id, table)
}

fn pack_undertaker_pending_attack(row_id: i32, table: i32) -> u64 {
    if row_id < 0 || !(0..=1).contains(&table) {
        return UNDERTAKER_PENDING_UNKNOWN_ATTACK;
    }
    // One atomic value publishes both fields: 0=empty, 1=unknown, tags 2/3=NPC/PC.
    ((row_id as u64) << 2) | (table as u64 + 2)
}

fn unpack_undertaker_pending_attack(pending_hit: u64) -> Option<(u32, u32)> {
    let row_id = pending_hit >> 2;
    let tag = pending_hit & 3;
    if row_id > i32::MAX as u64 || tag < 2 {
        return None;
    }
    Some(((tag - 2) as u32, row_id as u32))
}

fn undertaker_dodge_effects(
    pending_hit: u64,
    read_direction: impl FnOnce(u32, u32) -> Option<i8>,
) -> [Option<i32>; 2] {
    if pending_hit == 0 {
        return [None, None];
    }
    let direction_effect = unpack_undertaker_pending_attack(pending_hit)
        .and_then(|(table, row_id)| read_direction(table, row_id))
        .and_then(|direction| usize::try_from(direction).ok())
        .and_then(|direction| SP_UNDERTAKER_AUTO_DODGE_DIRECTIONS.get(direction).copied());
    [direction_effect, Some(SP_UNDERTAKER_AUTO_DODGE_TRIGGER)]
}

fn clear_undertaker_dodge_effects(
    player: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> shikifsrs_runtime::Result<()> {
    for effect in
        std::iter::once(SP_UNDERTAKER_AUTO_DODGE_TRIGGER).chain(SP_UNDERTAKER_AUTO_DODGE_DIRECTIONS)
    {
        native_world::revalidate(player)?;
        // SAFETY: Original permitted callback retains the full effect/unlink/PARAM graph.
        unsafe { native_character::remove_effect(player.address, effect) }?;
    }
    Ok(())
}

fn apply_pending_undertaker_auto_dodge(
    player: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) {
    let pending_hit = UNDERTAKER_AUTO_DODGE_PENDING_HIT.load(Ordering::Acquire);
    if pending_hit == 0 || !UNDERTAKER_AUTO_DODGE_ARMED.load(Ordering::Acquire) {
        return;
    }
    if let Err(error) = native_world::revalidate(player) {
        eprintln!("ShikiFSRS HUD dodge target identity rejected: {error}");
        return;
    }
    if UNDERTAKER_AUTO_DODGE_DISPATCH_BLOCKED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    // The original input stays published before native entry and through errors or interruption.
    // Failed native receipts remain in the real character adapter; never replay this sequence.
    let effects = undertaker_dodge_effects(pending_hit, native_parameters::attack_direction);
    if let Err(error) = clear_undertaker_dodge_effects(player) {
        eprintln!("ShikiFSRS HUD dodge cleanup failed; intent and receipts retained: {error}");
        return;
    }
    for effect in effects.into_iter().flatten() {
        if let Err(error) = native_world::revalidate(player) {
            eprintln!("ShikiFSRS HUD dodge identity changed; intent retained: {error}");
            return;
        }
        // SAFETY: Same full exclusive effect/native allocator graph. Direction precedes request.
        if let Err(error) = unsafe { native_character::apply_effect(player.address, effect, true) }
        {
            eprintln!(
                "ShikiFSRS HUD dodge submission failed; intent and actual receipt retained: {error}"
            );
            return;
        }
    }
    if UNDERTAKER_AUTO_DODGE_PENDING_HIT
        .compare_exchange(pending_hit, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        eprintln!("ShikiFSRS HUD dodge intent changed; session remains closed");
        return;
    }
    UNDERTAKER_AUTO_DODGE_DISPATCH_BLOCKED.store(false, Ordering::Release);
    // Count the completed submission sequence, not native gameplay completion.
    UNDERTAKER_AUTO_DODGE_PULSES.fetch_add(1, Ordering::Relaxed);
}

fn undertaker_auto_dodge_probe() -> (bool, bool, bool, u32, u32) {
    (
        UNDERTAKER_AUTO_DODGE_AVAILABLE.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_ARMED.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_SENSOR_ACTIVE.load(Ordering::Acquire),
        UNDERTAKER_AUTO_DODGE_CONTACTS.load(Ordering::Relaxed),
        UNDERTAKER_AUTO_DODGE_PULSES.load(Ordering::Relaxed),
    )
}

fn install_revenant_summon_range_hook_once() {
    INSTALL_REVENANT_SUMMON_RANGE_HOOK.call_once(|| {
        let hook_site = match native_hooks::address(2) {
            Ok(address) => address,
            Err(error) => {
                append_damage_log(&format!("[init] fixed Shiki hook site rejected: {error}"));
                return;
            }
        };
        let hook = unsafe {
            native_hooks::install_back(2, |regs: *mut ilhook::x64::Registers| {
                if !REVENANT_SUMMON_RANGE_ACTIVE.load(Ordering::Relaxed) {
                    return;
                }
                let range_data = (*regs).rax as usize;
                if range_data == 0 {
                    return;
                }
                if *(range_data as *const u32) != REVENANT_SUMMON_RANGE_BUDDY_TYPE_ID {
                    return;
                }

                let override_bits = REVENANT_SUMMON_RANGE_OVERRIDE.to_bits() as u128;
                (*regs).xmm2 = ((*regs).xmm2 & !0xFFFF_FFFFu128) | override_bits;
            })
        };

        match hook {
            Ok(()) => {
                append_damage_log(&format!(
                    "[init] revenant summon range hook installed at 0x{hook_site:X}"
                ));
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] revenant summon range hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

fn install_camera_directed_position_hook_once() {
    INSTALL_CAMERA_DIRECTED_POSITION_HOOK.call_once(|| {
        let hook_site = match native_hooks::address(3) {
            Ok(address) => address,
            Err(error) => {
                append_damage_log(&format!("[init] fixed Shiki hook site rejected: {error}"));
                return;
            }
        };
        let hook = unsafe {
            native_hooks::install_back(3, |regs: *mut ilhook::x64::Registers| {
                apply_camera_directed_position_hook(regs);
            })
        };

        match hook {
            Ok(()) => {
                append_damage_log(&format!(
                    "[init] camera directed position hook installed at 0x{hook_site:X}"
                ));
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] camera directed position hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

fn install_multi_lock_bullet_hook_once() {
    INSTALL_MULTI_LOCK_BULLET_HOOK.call_once(|| {
        let hook_site = match native_hooks::address(4) {
            Ok(address) => address,
            Err(error) => {
                append_damage_log(&format!("[init] fixed Shiki hook site rejected: {error}"));
                return;
            }
        };
        let hook = unsafe {
            native_hooks::install_back(4, |regs: *mut ilhook::x64::Registers| {
                apply_bullet_spawn_hook(regs);
            })
        };

        match hook {
            Ok(()) => {
                append_damage_log(&format!(
                    "[init] multi lock bullet hook installed at 0x{hook_site:X}"
                ));
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] multi lock bullet hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

fn set_revenant_summon_range_hook_active(active: bool) {
    REVENANT_SUMMON_RANGE_ACTIVE.store(active, Ordering::Relaxed);
}

#[cfg(test)]
fn find_unique_pattern_in_range(
    image: &[u8],
    start: usize,
    end: usize,
    pattern: &[Option<u8>],
) -> Option<usize> {
    if pattern.is_empty() || end <= start || end - start < pattern.len() {
        return None;
    }
    let mut matches = image[start..end]
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| {
            window
                .iter()
                .zip(pattern.iter())
                .all(|(byte, expected)| expected.is_none_or(|expected| *byte == expected))
        })
        .map(|(offset, _)| start + offset);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

fn apply_bullet_spawn_hook(regs: *mut ilhook::x64::Registers) {
    if regs.is_null() {
        return;
    }

    // Native hook input is copied through the independent consumed-span reader.
    let manager = unsafe { (*regs).rcx };
    let input_address = unsafe { (*regs).r8 };
    if manager == 0
        || input_address == 0
        || MULTI_LOCK_BULLET_SPAWNING_CLONE.load(Ordering::Relaxed)
    {
        return;
    }
    let source = match native_bullets::capture(input_address) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("ShikiFSRS HUD spawn input rejected: {error}");
            return;
        }
    };
    let bullet_id = source.input.bullet_parameter_id();
    if bullet_id < 0 {
        return;
    }
    let owner = HudHandle::from_native(source.input.owner_identifier());
    let original_target = HudHandle::from_native(source.input.target_identifier());
    let Some(target_plan) = multi_lock_bullet_targets(owner, original_target, manager) else {
        return;
    };

    record_multi_lock_spawn_hook_bullet_param(bullet_id);
    let targets = target_plan.targets;
    if targets.is_empty() {
        return;
    }

    let cursor = MULTI_LOCK_BULLET_TARGET_CURSOR.fetch_add(1, Ordering::Relaxed);
    let target = targets[cursor % targets.len()];
    if target != original_target {
        // SAFETY: Original spawn callback retains the full target/allocator graph and writable input.
        if let Err(error) = unsafe { native_bullets::retarget(source.clone(), target.raw()) } {
            eprintln!("ShikiFSRS HUD spawn retarget retained: {error}");
            return;
        }
    }
    if target_plan.spawn_extra_clones {
        spawn_multi_lock_extra_bullets(manager, &source, &targets, target);
    }
}

struct MultiLockBulletTargetPlan {
    targets: Vec<HudHandle>,
    spawn_extra_clones: bool,
}

#[derive(Clone, Copy)]
struct MultiLockConfig {
    target_limit: usize,
    marker_sp_effect: i32,
}

fn multi_lock_bullet_targets(
    owner: HudHandle,
    original_target: HudHandle,
    bullet_manager: u64,
) -> Option<MultiLockBulletTargetPlan> {
    let Ok(world_chr_man) = HudWorld::observe() else {
        return None;
    };
    let player = world_chr_man.player?;
    let owner_is_player = owner == player.actor.handle;
    let owner_belongs_to_player = owner_is_player
        || bullet_owner_belongs_to_player(owner, player.actor.handle, bullet_manager);
    let weapon_behavior_fallback = !owner_belongs_to_player
        && multi_lock_weapon_behavior_fallback(&world_chr_man, &player, &owner, original_target);
    if !owner_belongs_to_player && !weapon_behavior_fallback {
        return None;
    }

    multi_lock_targets_for_player(&world_chr_man, &player, original_target).map(|targets| {
        MultiLockBulletTargetPlan {
            targets,
            spawn_extra_clones: owner_is_player || weapon_behavior_fallback,
        }
    })
}

fn multi_lock_weapon_behavior_fallback(
    world_chr_man: &HudWorld,
    player: &HudPlayer,
    owner: &HudHandle,
    original_target: HudHandle,
) -> bool {
    if owner.is_bullet() || original_target.is_empty() {
        return false;
    }

    if original_target == player.actor.handle {
        return false;
    }

    let Some(target) = hud_actor_by_handle(world_chr_man, &original_target) else {
        return false;
    };
    if scholar_reject_reason(&target).is_some() {
        return false;
    }
    let Some(screen_pos) = scholar_target_screen_pos(&target) else {
        return false;
    };
    if !multi_lock_screen_pos_in_lens(screen_pos) {
        return false;
    }
    let player_pos = hud_physics_position(&player.actor);
    let target_pos = scholar_chr_physics_position(&target);
    let target_distance = distance_havok(player_pos, target_pos);
    scholar_in_lock_target_volume(player, target_pos, target_distance)
        && scholar_has_line_of_sight(&target, player)
}

struct MultiLockCloneGuard;
impl Drop for MultiLockCloneGuard {
    fn drop(&mut self) {
        MULTI_LOCK_BULLET_SPAWNING_CLONE.store(false, Ordering::Relaxed);
    }
}
fn spawn_multi_lock_extra_bullets(
    manager: u64,
    source: &shikifsrs_elden_ring::bullet_spawn::BulletSpawnObservation,
    targets: &[HudHandle],
    primary_target: HudHandle,
) {
    if targets.len() <= 1 {
        return;
    }
    if MULTI_LOCK_BULLET_SPAWNING_CLONE.swap(true, Ordering::Relaxed) {
        return;
    }
    let _guard = MultiLockCloneGuard;
    for target in targets
        .iter()
        .copied()
        .filter(|target| *target != primary_target)
        .take(MULTI_LOCK_BULLET_EXTRA_CLONES_PER_SPAWN)
    {
        let mut cloned = source.clone();
        cloned.input = cloned.input.with_target(target.raw());
        // SAFETY: Captured consumed fields retain the original callback's opaque references.
        // Native recursion reaches apply_bullet_spawn_hook and returns before borrowing this session.
        if let Err(error) = unsafe { native_bullets::spawn(manager, cloned) } {
            eprintln!("ShikiFSRS HUD clone spawn retained: {error}");
            break;
        }
    }
}

fn update_multi_lock_live_bullets() {
    let Ok(world_chr_man) = HudWorld::observe() else {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        return;
    };
    let Some(player) = world_chr_man.player else {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        return;
    };
    if multi_lock_target_limit(&player.actor).is_none() {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        MULTI_LOCK_LIVE_GUIDED_BULLET_COUNT.store(0, Ordering::Relaxed);
        if let Ok(mut seen) = MULTI_LOCK_LIVE_BULLET_SEEN.lock() {
            seen.clear();
        }
        if let Ok(mut guidance) = MULTI_LOCK_LIVE_WEAPON_GUIDANCE.lock() {
            guidance.clear();
        }
        return;
    }
    let target_handles =
        multi_lock_targets_for_player(&world_chr_man, &player, player.locked_target)
            .unwrap_or_default();
    let Ok(bullet_list) = native_bullets::observe() else {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        return;
    };

    let mut current = HashSet::new();
    let previous = MULTI_LOCK_LIVE_BULLET_SEEN
        .lock()
        .map(|seen| seen.clone())
        .unwrap_or_default();
    let now = Instant::now();
    let mut guided_count = 0u32;
    let mut guidance = MULTI_LOCK_LIVE_WEAPON_GUIDANCE.lock().ok();

    for bullet in &bullet_list.members {
        let handle = native_bullets::key(bullet);
        current.insert(handle);
        let param_id = multi_lock_bullet_param_id(bullet);
        let is_new = !previous.contains(&handle);
        let can_assign_guidance = is_new
            || f32::from_bits(bullet.elapsed_step_bits)
                <= MULTI_LOCK_LIVE_WEAPON_ASSIGN_WINDOW_SECONDS;
        let guided_target = if can_assign_guidance
            && guidance
                .as_ref()
                .is_none_or(|guidance| !guidance.contains_key(&handle))
        {
            assign_multi_lock_live_weapon_bullet_target(
                player.actor.handle,
                &target_handles,
                bullet,
                param_id,
            )
            .inspect(|target| {
                if let Some(guidance) = guidance.as_mut() {
                    guidance.insert(
                        handle,
                        MultiLockLiveWeaponGuidance {
                            target: *target,
                            created_at: now,
                            expires_at: now
                                + Duration::from_secs_f32(MULTI_LOCK_LIVE_WEAPON_GUIDANCE_SECONDS),
                        },
                    );
                }
            })
            .map(|target| (target, MULTI_LOCK_LIVE_WEAPON_GUIDANCE_MIX))
        } else {
            guidance.as_ref().and_then(|guidance| {
                guidance.get(&handle).and_then(|entry| {
                    (entry.expires_at > now).then(|| {
                        let total = MULTI_LOCK_LIVE_WEAPON_GUIDANCE_SECONDS.max(0.001);
                        let elapsed = now.duration_since(entry.created_at).as_secs_f32();
                        let fade = (1.0 - elapsed / total).clamp(0.0, 1.0);
                        (entry.target, MULTI_LOCK_LIVE_WEAPON_GUIDANCE_MIX * fade)
                    })
                })
            })
        };

        if let Some((target, mix)) = guided_target
            && apply_multi_lock_live_weapon_bullet_direction(&world_chr_man, bullet, target, mix) {
                guided_count += 1;
            }
    }

    MULTI_LOCK_LIVE_BULLET_COUNT.store(current.len() as u32, Ordering::Relaxed);
    if let Some(guidance) = guidance.as_mut() {
        guidance.retain(|handle, entry| entry.expires_at > now && current.contains(handle));
        guided_count = guided_count.max(guidance.len() as u32);
    }
    MULTI_LOCK_LIVE_GUIDED_BULLET_COUNT.store(guided_count, Ordering::Relaxed);
    if let Ok(mut seen) = MULTI_LOCK_LIVE_BULLET_SEEN.lock() {
        *seen = current;
    }
}

fn assign_multi_lock_live_weapon_bullet_target(
    player_handle: HudHandle,
    targets: &[HudHandle],
    bullet: &shikifsrs_elden_ring::bullets::BulletObservation,
    param_id: i32,
) -> Option<HudHandle> {
    if !multi_lock_should_guide_live_bullet_param(param_id)
        || bullet.targeting_owner_identifier != player_handle.raw()
        || targets.is_empty()
    {
        return None;
    }

    let cursor = MULTI_LOCK_BULLET_TARGET_CURSOR.fetch_add(1, Ordering::Relaxed);
    Some(targets[cursor % targets.len()])
}

fn record_multi_lock_spawn_hook_bullet_param(param_id: i32) {
    if param_id < 0 {
        return;
    }
    if let Ok(mut params) = MULTI_LOCK_SPAWN_HOOK_BULLET_PARAMS.lock() {
        let now = Instant::now();
        params.insert(
            param_id,
            now + Duration::from_secs_f32(MULTI_LOCK_SPAWN_HOOK_PARAM_SUPPRESS_SECONDS),
        );
        params.retain(|_, expires_at| *expires_at > now);
    }
}

fn multi_lock_should_guide_live_bullet_param(param_id: i32) -> bool {
    if param_id < 0 {
        return false;
    }
    if param_id >= MULTI_LOCK_LIVE_ALWAYS_GUIDE_PARAM_MIN {
        return true;
    }

    let now = Instant::now();
    let Ok(mut params) = MULTI_LOCK_SPAWN_HOOK_BULLET_PARAMS.lock() else {
        return true;
    };
    params.retain(|_, expires_at| *expires_at > now);
    !params
        .get(&param_id)
        .is_some_and(|expires_at| *expires_at > now)
}

fn apply_multi_lock_live_weapon_bullet_direction(
    world_chr_man: &HudWorld,
    bullet: &shikifsrs_elden_ring::bullets::BulletObservation,
    target_handle: HudHandle,
    mix: f32,
) -> bool {
    let Some(target) = hud_actor_by_handle(world_chr_man, &target_handle) else {
        return false;
    };
    let Some(target_pos) = scholar_target_havok_position(&target, 0.55) else {
        return false;
    };
    let Some(xyz) =
        native_bullets::guidance_velocity(bullet, [target_pos.0, target_pos.1, target_pos.2], mix)
    else {
        return false;
    };
    // SAFETY: Actual callback retains this exact bullet lifetime and exclusive writable XYZ extent.
    match unsafe { native_bullets::velocity(*bullet, xyz) } {
        Ok(()) => true,
        Err(error) => {
            eprintln!("ShikiFSRS HUD bullet velocity retained: {error}");
            false
        }
    }
}

fn multi_lock_bullet_param_id(bullet: &shikifsrs_elden_ring::bullets::BulletObservation) -> i32 {
    let fly = bullet.fly.parameter_id;
    if fly >= 0 {
        return fly;
    }
    let wait = bullet.wait.parameter_id;
    if wait >= 0 {
        return wait;
    }
    bullet.explosion.parameter_id
}

fn multi_lock_snapshot_targets() -> (
    bool,
    [MultiLockTargetSnapshot; MULTI_LOCK_BULLET_MAX_TARGETS],
) {
    let mut snapshot = [MultiLockTargetSnapshot::default(); MULTI_LOCK_BULLET_MAX_TARGETS];
    let Ok(world_chr_man) = HudWorld::observe() else {
        return (false, snapshot);
    };
    let Some(player) = world_chr_man.player else {
        refresh_multi_lock_target_markers(&[], None);
        return (false, snapshot);
    };
    let Some(config) = multi_lock_config(&player.actor) else {
        refresh_multi_lock_target_markers(&[], None);
        return (false, snapshot);
    };

    if let Some(targets) =
        multi_lock_targets_for_player(&world_chr_man, &player, player.locked_target)
    {
        refresh_multi_lock_target_markers(&targets, Some(config.marker_sp_effect));
        for (index, (slot, handle)) in snapshot.iter_mut().zip(targets.iter()).enumerate() {
            let Some(pos) = hud_actor_by_handle(&world_chr_man, handle)
                .and_then(|actor| scholar_target_screen_pos(&actor))
            else {
                continue;
            };
            *slot = MultiLockTargetSnapshot {
                active: true,
                pos,
                primary: index == 0,
            };
        }
    } else {
        refresh_multi_lock_target_markers(&[], None);
    }

    (true, snapshot)
}

fn refresh_multi_lock_target_markers(targets: &[HudHandle], marker_sp_effect: Option<i32>) {
    let Ok(mut marker_state) = MULTI_LOCK_TARGET_MARKERS.lock() else {
        return;
    };
    let Ok(mut copied_world) = HudWorld::observe() else {
        return;
    };
    let world_chr_man = &mut copied_world;

    let current_targets: HashSet<HudHandle> = targets.iter().copied().collect();
    let previous: Vec<(HudHandle, i32)> = marker_state
        .iter()
        .map(|(handle, marker)| (*handle, *marker))
        .collect();

    for (handle, old_marker) in previous {
        let should_remove =
            marker_sp_effect != Some(old_marker) || !current_targets.contains(&handle);
        if should_remove {
            if let Some(chr) = hud_actor_by_handle(world_chr_man, &handle) {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe { native_character::remove_effect(chr.identity.address, old_marker) },
                );
            }
            marker_state.remove(&handle);
        }
    }

    let Some(marker_sp_effect) = marker_sp_effect else {
        return;
    };
    for handle in targets {
        if let Some(chr) = hud_actor_by_handle(world_chr_man, handle) {
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(chr.identity.address, marker_sp_effect, true)
                },
            );
            marker_state.insert(*handle, marker_sp_effect);
        }
    }
}

fn bullet_owner_belongs_to_player(
    owner: HudHandle,
    player_handle: HudHandle,
    bullet_manager: u64,
) -> bool {
    if owner == player_handle {
        return true;
    }
    if !owner.is_bullet() || bullet_manager == 0 {
        return false;
    }
    match native_bullets::observe() {
        Ok(list) if list.manager_address == bullet_manager => list.members.iter().any(|bullet| {
            bullet.identity.native_identifier == owner.raw()
                && bullet.targeting_owner_identifier == player_handle.raw()
        }),
        Ok(_) => false,
        Err(error) => {
            eprintln!("ShikiFSRS HUD bullet owner rejected: {error}");
            false
        }
    }
}

fn multi_lock_targets_for_player(
    world_chr_man: &HudWorld,
    player: &HudPlayer,
    original_target: HudHandle,
) -> Option<Vec<HudHandle>> {
    let config = multi_lock_config(&player.actor)?;
    let target_limit = config.target_limit;

    let mut targets = Vec::with_capacity(target_limit);
    push_multi_lock_target(&mut targets, original_target, target_limit, world_chr_man);
    push_multi_lock_target(
        &mut targets,
        player.locked_target,
        target_limit,
        world_chr_man,
    );

    let player_pos = hud_physics_position(&player.actor);
    let entries = match native_world::update_entries(world_chr_man.object.address) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
            return None;
        }
    };
    for entry in entries.into_iter().take(MULTI_LOCK_BULLET_SCAN_MAX_ENTRIES) {
        if targets.len() >= target_limit {
            break;
        }

        if let Err(error) = native_world::revalidate(entry.character) {
            eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
            return None;
        }
        // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
        // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
        let copied = match HudActor::observe(entry.character) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                return None;
            }
        };
        let chr = &copied;
        if targets.contains(&chr.handle) || scholar_reject_reason(chr).is_some() {
            continue;
        }
        let Some(screen_pos) = scholar_target_screen_pos(chr) else {
            continue;
        };
        if !multi_lock_screen_pos_in_lens(screen_pos) {
            continue;
        }
        let target_pos = scholar_chr_physics_position(chr);
        let target_distance = distance_havok(player_pos, target_pos);
        if !scholar_in_lock_target_volume(player, target_pos, target_distance) {
            continue;
        }
        if !scholar_has_line_of_sight(chr, player) {
            continue;
        }
        targets.push(chr.handle);
    }

    Some(targets)
}

fn multi_lock_target_limit(player: &HudActor) -> Option<usize> {
    multi_lock_config(player).map(|config| config.target_limit)
}

fn multi_lock_config(player: &HudActor) -> Option<MultiLockConfig> {
    native_character::effects(player.identity.address)
        .map_err(|error| {
            eprintln!("ShikiFSRS HUD multi-lock effects rejected: {error}");
        })
        .ok()?
        .into_iter()
        .filter_map(multi_lock_config_from_sp_effect)
        .max_by_key(|config| (config.target_limit, config.marker_sp_effect))
}

fn multi_lock_config_from_sp_effect(sp_effect: i32) -> Option<MultiLockConfig> {
    if !(MULTI_LOCK_PLAYER_EFFECT_START..MULTI_LOCK_PLAYER_EFFECT_END).contains(&sp_effect) {
        return None;
    }
    let relative = sp_effect - MULTI_LOCK_PLAYER_EFFECT_START;
    let tier_index = relative % MULTI_LOCK_PLAYER_EFFECT_GROUP_SIZE;
    let style_index = relative / MULTI_LOCK_PLAYER_EFFECT_GROUP_SIZE;
    let target_limit = MULTI_LOCK_BULLET_TARGET_TIERS
        .get(tier_index as usize)
        .copied()?;
    Some(MultiLockConfig {
        target_limit,
        marker_sp_effect: MULTI_LOCK_TARGET_MARKER_START + style_index,
    })
}

fn multi_lock_screen_pos_in_lens(pos: [f32; 2]) -> bool {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return true;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return true;
    }
    let center = [width * SCHOLAR_LENS_CENTER_X_FACTOR, height * 0.5];
    let radius =
        height.min(width) * SCHOLAR_LENS_RADIUS_FACTOR * MULTI_LOCK_SCREEN_LENS_RADIUS_SCALE;
    let dx = pos[0] - center[0];
    let dy = pos[1] - center[1];
    dx * dx + dy * dy <= radius * radius
}

fn push_multi_lock_target(
    targets: &mut Vec<HudHandle>,
    target: HudHandle,
    target_limit: usize,
    world_chr_man: &HudWorld,
) {
    if targets.len() >= target_limit || target.is_empty() || targets.contains(&target) {
        return;
    }
    let Some(chr) = hud_actor_by_handle(world_chr_man, &target) else {
        return;
    };
    if scholar_reject_reason(&chr).is_none() {
        targets.push(target);
    }
}

fn apply_camera_directed_position_hook(regs: *mut ilhook::x64::Registers) {
    let Some(player_physics) = active_camera_directed_player_physics() else {
        reset_camera_directed_vertical_bias();
        return;
    };
    let physics = unsafe { (*regs).rbx as usize };
    if physics == 0 || physics != player_physics {
        return;
    }

    let old_x = unsafe { read_f32(physics + CAMERA_DIRECTED_POSITION_OFFSET) };
    let old_z = unsafe { read_f32(physics + CAMERA_DIRECTED_POSITION_OFFSET + 0x8) };
    let next_x = xmm_f32(unsafe { (*regs).xmm0 }, 0);
    let native_next_y = xmm_f32(unsafe { (*regs).xmm0 }, 1);
    let next_z = xmm_f32(unsafe { (*regs).xmm0 }, 2);
    let native_dx = next_x - old_x;
    let native_dz = next_z - old_z;
    let horizontal_len = (native_dx * native_dx + native_dz * native_dz).sqrt();
    if !native_next_y.is_finite()
        || !next_x.is_finite()
        || !next_z.is_finite()
        || !horizontal_len.is_finite()
        || !(CAMERA_DIRECTED_MIN_FRAME_DELTA..=CAMERA_DIRECTED_MAX_FRAME_DELTA).contains(&horizontal_len)
    {
        decay_camera_directed_vertical_bias();
        return;
    }

    let Some(camera_pitch) = camera_pitch_direction() else {
        decay_camera_directed_vertical_bias();
        return;
    };
    let bias = update_camera_directed_vertical_bias(camera_pitch, horizontal_len);
    let (mixed_x, mixed_z) = camera_directed_mixed_horizontal_position(
        old_x,
        old_z,
        native_dx,
        native_dz,
        horizontal_len,
        camera_pitch,
    )
    .unwrap_or((next_x, next_z));
    let next_y = native_next_y + bias;
    unsafe {
        (*regs).xmm0 = set_xmm_f32((*regs).xmm0, 0, mixed_x);
        (*regs).xmm0 = set_xmm_f32((*regs).xmm0, 1, next_y);
        (*regs).xmm0 = set_xmm_f32((*regs).xmm0, 2, mixed_z);
    }
}

fn camera_directed_mixed_horizontal_position(
    old_x: f32,
    old_z: f32,
    native_dx: f32,
    native_dz: f32,
    horizontal_len: f32,
    camera_pitch: f32,
) -> Option<(f32, f32)> {
    let pitch_strength = camera_directed_pitch_strength(camera_pitch);
    if pitch_strength <= 0.0 {
        return None;
    }

    let (camera_x, camera_z) = camera_horizontal_direction()?;
    let mix = pitch_strength * CAMERA_DIRECTED_HORIZONTAL_MIX_MAX;
    let mixed_dx = native_dx * (1.0 - mix) + camera_x * horizontal_len * mix;
    let mixed_dz = native_dz * (1.0 - mix) + camera_z * horizontal_len * mix;
    Some((old_x + mixed_dx, old_z + mixed_dz))
}

fn update_camera_directed_vertical_bias(camera_pitch: f32, _horizontal_len: f32) -> f32 {
    let Ok(mut bias) = CAMERA_DIRECTED_VERTICAL_BIAS.lock() else {
        return 0.0;
    };
    let pitch_strength = camera_directed_pitch_strength(camera_pitch);
    if pitch_strength <= 0.0 {
        *bias *= CAMERA_DIRECTED_VERTICAL_BIAS_DECAY;
    } else {
        let curved_pitch = pitch_strength * pitch_strength;
        let target_limit = if camera_pitch > 0.0 {
            CAMERA_DIRECTED_VERTICAL_BIAS_MAX
        } else {
            -CAMERA_DIRECTED_VERTICAL_BIAS_MIN
        };
        let target_bias =
            camera_pitch.signum() * curved_pitch * target_limit * CAMERA_DIRECTED_VERTICAL_SCALE;
        let delta = (target_bias - *bias).clamp(
            -CAMERA_DIRECTED_MAX_VERTICAL_DELTA,
            CAMERA_DIRECTED_MAX_VERTICAL_DELTA,
        );
        *bias = (*bias + delta * CAMERA_DIRECTED_VERTICAL_BIAS_RESPONSE).clamp(
            CAMERA_DIRECTED_VERTICAL_BIAS_MIN,
            CAMERA_DIRECTED_VERTICAL_BIAS_MAX,
        );
    }
    if bias.abs() < 0.001 {
        *bias = 0.0;
    }
    *bias
}

fn camera_directed_pitch_strength(camera_pitch: f32) -> f32 {
    ((camera_pitch.abs() - CAMERA_DIRECTED_PITCH_DEADZONE)
        / (CAMERA_DIRECTED_HORIZONTAL_MIX_FULL_PITCH - CAMERA_DIRECTED_PITCH_DEADZONE))
        .clamp(0.0, 1.0)
}

fn decay_camera_directed_vertical_bias() {
    let Ok(mut bias) = CAMERA_DIRECTED_VERTICAL_BIAS.lock() else {
        return;
    };
    *bias *= CAMERA_DIRECTED_VERTICAL_BIAS_DECAY;
    if bias.abs() < 0.001 {
        *bias = 0.0;
    }
}

fn reset_camera_directed_vertical_bias() {
    if let Ok(mut bias) = CAMERA_DIRECTED_VERTICAL_BIAS.lock() {
        *bias = 0.0;
    }
}

fn camera_horizontal_direction() -> Option<(f32, f32)> {
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return None;
    };
    let forward = camera.pers_cam_1.forward();
    let len = (forward.0 * forward.0 + forward.2 * forward.2).sqrt();
    if !len.is_finite() || len < 0.001 {
        return None;
    }
    Some((forward.0 / len, forward.2 / len))
}

fn active_camera_directed_player_physics() -> Option<usize> {
    let Ok(world_chr_man) = HudWorld::observe() else {
        return None;
    };
    let player = world_chr_man.player?;
    if !chr_has_speffect(&player.actor, CAMERA_DIRECTED_MOVEMENT_EFFECT) {
        return None;
    }
    native_character::module_address(
        player.actor.identity.address,
        shikifsrs_elden_ring::character_modules::CharacterModule::Physics,
    )
    .ok()
    .map(|address| address as usize)
}

unsafe fn read_f32(address: usize) -> f32 {
    unsafe { std::ptr::read_unaligned(address as *const f32) }
}

unsafe fn read_i32(address: usize) -> i32 {
    unsafe { std::ptr::read_unaligned(address as *const i32) }
}

fn xmm_f32(xmm: u128, lane: usize) -> f32 {
    f32::from_bits(((xmm >> (lane * 32)) & 0xFFFF_FFFF) as u32)
}

fn set_xmm_f32(xmm: u128, lane: usize, value: f32) -> u128 {
    let shift = lane * 32;
    let mask = !(0xFFFF_FFFFu128 << shift);
    (xmm & mask) | ((value.to_bits() as u128) << shift)
}

fn record_duchess_replay_event(target_damage_module: usize, payload: DuchessReplayPayload) {
    if target_damage_module == 0 || !payload.has_effect() {
        return;
    }
    let local_damage_module = LOCAL_DAMAGE_MODULE.load(Ordering::Relaxed);
    if local_damage_module != 0 && target_damage_module == local_damage_module {
        return;
    }
    let payload = consume_duchess_replay_ignore(target_damage_module, payload);
    if !payload.has_effect() {
        return;
    }

    let now = Instant::now();
    if let Ok(mut events) = DUCHESS_REPLAY_HISTORY.lock() {
        events.retain(|event| {
            (now - event.created_at).as_secs_f32() <= DUCHESS_REPLAY_HISTORY_SECONDS
        });
        if events.len() >= DUCHESS_REPLAY_HISTORY_CAP {
            let overflow = events.len() + 1 - DUCHESS_REPLAY_HISTORY_CAP;
            events.drain(0..overflow);
        }
        events.push(DuchessReplayEvent {
            target_damage_module,
            payload,
            created_at: now,
        });
    }
}

fn duchess_replay_payload_for(target_damage_module: usize, now: Instant) -> DuchessReplayPayload {
    if target_damage_module == 0 {
        return DuchessReplayPayload::default();
    }
    DUCHESS_REPLAY_HISTORY
        .lock()
        .map(|mut events| {
            events.retain(|event| {
                (now - event.created_at).as_secs_f32() <= DUCHESS_REPLAY_HISTORY_SECONDS
            });
            let mut payload = DuchessReplayPayload::default();
            for event in events
                .iter()
                .filter(|event| event.target_damage_module == target_damage_module)
            {
                payload.add(event.payload);
            }
            payload
        })
        .unwrap_or_default()
}

fn add_duchess_replay_ignore(target_damage_module: usize, payload: DuchessReplayPayload) {
    if target_damage_module == 0 || !payload.has_effect() {
        return;
    }
    if let Ok(mut ignores) = DUCHESS_REPLAY_IGNORES.lock() {
        ignores
            .entry(target_damage_module)
            .and_modify(|entry| entry.add(payload))
            .or_insert(payload);
    }
}

fn consume_duchess_replay_ignore(
    target_damage_module: usize,
    payload: DuchessReplayPayload,
) -> DuchessReplayPayload {
    if target_damage_module == 0 || !payload.has_effect() {
        return payload;
    }
    let Ok(mut ignores) = DUCHESS_REPLAY_IGNORES.lock() else {
        return payload;
    };
    let Some(ignored) = ignores.get_mut(&target_damage_module) else {
        return payload;
    };

    let mut remaining = payload;
    let consumed_hp = remaining.hp_damage.min(ignored.hp_damage);
    remaining.hp_damage -= consumed_hp;
    ignored.hp_damage -= consumed_hp;

    for (remaining_status, ignored_status) in
        remaining.status.iter_mut().zip(ignored.status.iter_mut())
    {
        let consumed = (*remaining_status).min(*ignored_status);
        *remaining_status -= consumed;
        *ignored_status -= consumed;
    }

    let consumed_super_armor = remaining.super_armor_damage.min(ignored.super_armor_damage);
    remaining.super_armor_damage -= consumed_super_armor;
    ignored.super_armor_damage -= consumed_super_armor;

    if !ignored.has_effect() {
        ignores.remove(&target_damage_module);
    }
    remaining
}

fn scale_i32(value: i32, rate: f32) -> i32 {
    ((value.max(0) as f32) * rate).floor() as i32
}

fn record_damage_hook_event(target_damage_module: usize, damage: i32) {
    if target_damage_module == 0 || damage <= 0 {
        return;
    }
    let local_damage_module = LOCAL_DAMAGE_MODULE.load(Ordering::Relaxed);
    if local_damage_module != 0 && target_damage_module == local_damage_module {
        return;
    }
    if let Ok(mut events) = DAMAGE_HOOK_EVENTS.lock() {
        if events.len() >= 256 {
            let overflow = events.len() + 1 - 256;
            events.drain(0..overflow);
        }
        events.push(DamageHookEvent {
            target_damage_module,
            damage,
            created_at: Instant::now(),
        });
    }
}

fn drain_damage_hook_events() -> Vec<DamageHookEvent> {
    DAMAGE_HOOK_EVENTS
        .lock()
        .map(|mut events| events.drain(..).collect())
        .unwrap_or_default()
}

fn is_chargeable_damage_event(target_damage_module: usize, damage_data: usize) -> bool {
    if target_damage_module == 0 || damage_data == 0 {
        return false;
    }

    let local_damage_module = LOCAL_DAMAGE_MODULE.load(Ordering::Relaxed);
    if local_damage_module != 0 && target_damage_module == local_damage_module {
        DAMAGE_HOOK_EXCLUDED_LOCAL.fetch_add(1, Ordering::Relaxed);
        return false;
    }

    if let Some((offset, value)) = find_damage_candidate(damage_data) {
        DAMAGE_HOOK_POSITIVE.fetch_add(1, Ordering::Relaxed);
        DAMAGE_HOOK_LAST_OFFSET.store(offset as u32, Ordering::Relaxed);
        DAMAGE_HOOK_LAST_VALUE.store(value as u32, Ordering::Relaxed);
        true
    } else {
        false
    }
}

fn find_damage_candidate(damage_data: usize) -> Option<(usize, i32)> {
    for offset in DAMAGE_VALUE_OFFSETS {
        let value = unsafe { *((damage_data + offset) as *const i32) };
        if value > 0 {
            return Some((offset, value));
        }
    }

    let mut offset = DAMAGE_SCAN_START;
    while offset <= DAMAGE_SCAN_END {
        let value = unsafe { *((damage_data + offset) as *const i32) };
        if value > 0 && value < 1_000_000 {
            return Some((offset, value));
        }
        offset += 4;
    }

    None
}

fn find_recluse_damage_element(damage_data: usize) -> Option<RecluseElement> {
    let candidates = [
        (RecluseElement::Fire, unsafe {
            *((damage_data + 0x234) as *const i32)
        }),
        (RecluseElement::Magic, unsafe {
            *((damage_data + 0x238) as *const i32)
        }),
        (RecluseElement::Lightning, unsafe {
            *((damage_data + 0x23C) as *const i32)
        }),
        (RecluseElement::Holy, unsafe {
            *((damage_data + 0x240) as *const i32)
        }),
    ];

    candidates
        .into_iter()
        .filter(|(_, value)| *value > 0)
        .max_by_key(|(_, value)| *value)
        .map(|(element, _)| element)
}

fn record_recluse_damage_element(target_damage_module: usize, element: RecluseElement) {
    if target_damage_module == 0 {
        return;
    }
    if let Ok(mut elements) = RECLUSE_DAMAGE_ELEMENTS.lock() {
        elements.insert(target_damage_module, element);
    }
}

fn lookup_recluse_damage_element(target_damage_module: usize) -> Option<RecluseElement> {
    RECLUSE_DAMAGE_ELEMENTS
        .lock()
        .ok()
        .and_then(|elements| elements.get(&target_damage_module).copied())
}

fn clear_recluse_damage_element(target_damage_module: usize) {
    if target_damage_module == 0 {
        return;
    }
    if let Ok(mut elements) = RECLUSE_DAMAGE_ELEMENTS.lock() {
        elements.remove(&target_damage_module);
    }
}

fn damage_module_from_chr_ins(chr_ins: &HudActor) -> usize {
    native_character::module_address(
        chr_ins.identity.address,
        shikifsrs_elden_ring::character_modules::CharacterModule::Damage,
    )
    .map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD damage owner rejected: {error}");
            0
        },
        |address| address as usize,
    )
}

fn target_recluse_damage_module(
    world_chr_man: &HudWorld,
    locked_on_enemy: &HudHandle,
) -> Option<usize> {
    if locked_on_enemy.is_empty() {
        return None;
    }

    let target = hud_actor_by_handle(world_chr_man, locked_on_enemy)?;
    let damage_module = damage_module_from_chr_ins(&target);
    (damage_module != 0).then_some(damage_module)
}

fn locked_target_screen_pos(
    world_chr_man: &HudWorld,
    locked_on_enemy: &HudHandle,
) -> Option<[f32; 2]> {
    if locked_on_enemy.is_empty() {
        return None;
    }

    let projected = locked_target_body_screen_pos(world_chr_man, locked_on_enemy);

    projected.or_else(|| locked_target_tag_screen_pos(locked_on_enemy))
}

fn locked_target_body_screen_pos(
    world_chr_man: &HudWorld,
    locked_on_enemy: &HudHandle,
) -> Option<[f32; 2]> {
    let target = hud_actor_by_handle(world_chr_man, locked_on_enemy)?;
    let camera = native_camera::HudCamera::observe().ok()?;
    let body_position = recluse_body_marker_position(&target)?;
    project_lock_on_position(body_position, camera.pers_cam_1.as_ref())
}

fn recluse_body_marker_position(target: &HudActor) -> Option<native_camera::Vector4> {
    let geometry = native_character::marker_geometry(target.identity).ok()?;
    let base = geometry.model_base;
    let offset = geometry.frontend_displacement;
    let lift = recluse_body_marker_lift(target, base[1])?;

    Some(native_camera::Vector4(
        base[0] + offset[0],
        base[1] + lift,
        base[2] + offset[2],
        1.0,
    ))
}

fn recluse_body_marker_lift(target: &HudActor, base_y: f32) -> Option<f32> {
    let physics = native_character::physics_shape(target.identity.address).ok()?;
    let hit_lift = physics.height * RECLUSE_MARKER_HIT_HEIGHT_FACTOR;
    let [minimum, maximum] = native_character::model_bounds(target.identity.address).ok()??;
    let min_y = f32::from_bits(minimum[1]);
    let max_y = f32::from_bits(maximum[1]);
    let aabb_lift = (min_y + (max_y - min_y) * RECLUSE_MARKER_AABB_HEIGHT_FACTOR) - base_y;

    match (
        valid_recluse_marker_lift(hit_lift),
        valid_recluse_marker_lift(aabb_lift),
    ) {
        (true, true) => return Some(hit_lift.max(aabb_lift)),
        (true, false) => return Some(hit_lift),
        (false, true) => return Some(aabb_lift),
        (false, false) => {}
    }

    // Raw cache is a consumer fallback only; no lock-point identity is inferred.
    if let Ok(cache) = native_character::position_cache_d0(target.identity) {
        let cache_lift = cache[1] - base_y;
        if valid_recluse_marker_lift(cache_lift) { return Some(cache_lift); }
    }

    let scale_y = native_character::axis_scale(target.identity).ok()?[1];
    Some(RECLUSE_MARKER_FALLBACK_BODY_LIFT * scale_y.abs().clamp(0.5, 4.0))
}

fn valid_recluse_marker_lift(lift: f32) -> bool {
    lift.is_finite()
        && (RECLUSE_MARKER_MIN_LOCK_LIFT..=RECLUSE_MARKER_MAX_LOCK_LIFT).contains(&lift)
}

fn project_lock_on_position(
    position: native_camera::Vector4,
    camera: &native_camera::HudPerspective,
) -> Option<[f32; 2]> {
    project_lock_on_position_with_mode(position, camera, true)
}

fn project_lock_on_position_column(
    position: native_camera::Vector4,
    camera: &native_camera::HudPerspective,
) -> Option<[f32; 2]> {
    project_lock_on_position_with_mode(position, camera, false)
}

fn project_lock_on_position_with_mode(
    position: native_camera::Vector4,
    camera: &native_camera::HudPerspective,
    use_row_axes: bool,
) -> Option<[f32; 2]> {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return None;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let matrix = camera.matrix;
    let rel = [
        position.0 - matrix.3.0,
        position.1 - matrix.3.1,
        position.2 - matrix.3.2,
    ];

    let fov = camera.fov;
    if !fov.is_finite() || fov <= 0.0 {
        return None;
    }
    let fov_radians = if fov > std::f32::consts::PI {
        fov.to_radians()
    } else {
        fov
    };
    let tan_half_fov = (fov_radians * 0.5).tan();
    if tan_half_fov.abs() < 0.001 {
        return None;
    }
    let aspect = if camera.aspect_ratio.is_finite() && camera.aspect_ratio > 0.0 {
        camera.aspect_ratio
    } else {
        width / height
    };

    let row_axes = (
        [matrix.0.0, matrix.0.1, matrix.0.2],
        [matrix.1.0, matrix.1.1, matrix.1.2],
        [matrix.2.0, matrix.2.1, matrix.2.2],
    );
    let column_axes = (
        [matrix.0.0, matrix.1.0, matrix.2.0],
        [matrix.0.1, matrix.1.1, matrix.2.1],
        [matrix.0.2, matrix.1.2, matrix.2.2],
    );
    let axes = if use_row_axes { row_axes } else { column_axes };
    project_with_camera_axes(rel, axes, width, height, aspect, tan_half_fov)
}

fn locked_target_tag_screen_pos(locked_on_enemy: &HudHandle) -> Option<[f32; 2]> {
    locked_target_enemy_display_screen_pos(locked_on_enemy)
        .or_else(|| locked_target_frontend_tag_screen_pos(locked_on_enemy))
}

fn locked_target_enemy_display_screen_pos(locked_on_enemy: &HudHandle) -> Option<[f32; 2]> {
    native_hud::enemy_display_position(locked_on_enemy.raw())
        .map_err(|error| eprintln!("ShikiFSRS HUD display marker copy rejected: {error}"))
        .ok()
        .flatten()
        .and_then(valid_screen_pos)
}

fn locked_target_frontend_tag_screen_pos(locked_on_enemy: &HudHandle) -> Option<[f32; 2]> {
    native_hud::frontend_tag_position(locked_on_enemy.raw())
        .map_err(|error| eprintln!("ShikiFSRS HUD tag marker copy rejected: {error}"))
        .ok()
        .flatten()
        .and_then(valid_screen_pos)
}

fn locked_target_debug_screen_points(
    world_chr_man: &HudWorld,
    locked_on_enemy: &HudHandle,
) -> LockDebugPoints {
    if locked_on_enemy.is_empty() {
        return [None; 5];
    }

    let Some(target) = hud_actor_by_handle(world_chr_man, locked_on_enemy) else {
        return [None; 5];
    };
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return [None; 5];
    };
    let camera = camera.pers_cam_1.as_ref();
    // Keep five diagnostics, with explicitly named independent sources. Missing one
    // source does not erase the other actual frontend positions.
    let cache = native_character::position_cache_d0(target.identity).ok().map(|[x,y,z,w]| native_camera::Vector4(x,y,z,w));
    let anchor = native_character::marker_geometry(target.identity).ok().map(|copy| {
        let [x,y,z] = copy.frontend_anchor;
        native_camera::Vector4(x,y,z,1.0)
    });

    [
        cache.and_then(|point| project_lock_on_position(point, camera)),
        cache.and_then(|point| project_lock_on_position_column(point, camera)),
        anchor.and_then(|point| project_lock_on_position(point, camera)),
        locked_target_enemy_display_screen_pos(locked_on_enemy),
        locked_target_frontend_tag_screen_pos(locked_on_enemy),
    ]
}

fn scholar_target_screen_pos(target: &HudActor) -> Option<[f32; 2]> {
    let camera = native_camera::HudCamera::observe().ok()?;
    let body_position = scholar_body_marker_position(target)?;
    project_lock_on_position(body_position, camera.pers_cam_1.as_ref())
}

fn ironeye_weakness_marker_screen_pos(player: &HudPlayer, target: &HudActor) -> Option<[f32; 2]> {
    let player_pos = hud_physics_position(&player.actor);
    project_character_marker_screen_pos(player_pos, scholar_body_marker_position(target)?)
}

fn resurrection_accumulation_teammate_screen_pos(
    player_position: HudPosition,
    teammate: &HudActor,
) -> Option<[f32; 2]> {
    project_character_marker_screen_pos(
        player_position,
        resurrection_accumulation_teammate_marker_position(teammate)?,
    )
}

fn project_character_marker_screen_pos(
    player_position: HudPosition,
    target_position: native_camera::Vector4,
) -> Option<[f32; 2]> {
    let camera = native_camera::HudCamera::observe().ok()?;
    let camera = camera.pers_cam_1.as_ref();
    let camera_pos = camera.position();
    camera_depths_on_same_projection_side(
        [camera_pos.0, camera_pos.1, camera_pos.2],
        [camera.matrix.2.0, camera.matrix.2.1, camera.matrix.2.2],
        [player_position.0, player_position.1, player_position.2],
        [target_position.0, target_position.1, target_position.2],
    )?;

    project_lock_on_position(target_position, camera).filter(|pos| scholar_screen_pos_on_view(*pos))
}

fn scholar_body_marker_position(target: &HudActor) -> Option<native_camera::Vector4> {
    let offset = native_character::marker_geometry(target.identity).ok()?.frontend_displacement;
    let physics = native_character::physics_shape(target.identity.address).ok()?;
    let [x, y, z, w] = physics.position_bits.map(f32::from_bits);
    let base = HudPosition(x, y, z, w);
    let lift = (physics.height * RECLUSE_MARKER_HIT_HEIGHT_FACTOR)
        .clamp(RECLUSE_MARKER_MIN_LOCK_LIFT, RECLUSE_MARKER_MAX_LOCK_LIFT);

    Some(native_camera::Vector4(
        base.0 + offset[0],
        base.1 + lift,
        base.2 + offset[2],
        1.0,
    ))
}

fn scholar_screen_pos_in_lens(pos: [f32; 2]) -> bool {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return false;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return false;
    }
    let center = [width * SCHOLAR_LENS_CENTER_X_FACTOR, height * 0.5];
    let radius = height.min(width) * SCHOLAR_SCAN_RADIUS_FACTOR;
    screen_distance_sq(pos, center) <= radius * radius
}

fn scholar_screen_pos_on_view(pos: [f32; 2]) -> bool {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return false;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    width > 0.0
        && height > 0.0
        && pos[0].is_finite()
        && pos[1].is_finite()
        && pos[0] >= 0.0
        && pos[0] <= width
        && pos[1] >= 0.0
        && pos[1] <= height
}

fn scholar_scan_gain(distance: f32) -> f32 {
    if distance >= SCHOLAR_SCAN_MAX_DISTANCE {
        return 0.0;
    }
    if distance <= SCHOLAR_SCAN_CLOSE_DISTANCE {
        return SCHOLAR_SCAN_GAIN_MAX;
    }

    let distance_t = ((distance - SCHOLAR_SCAN_CLOSE_DISTANCE)
        / (SCHOLAR_SCAN_MAX_DISTANCE - SCHOLAR_SCAN_CLOSE_DISTANCE))
        .clamp(0.0, 1.0);
    let close_factor = (1.0 - distance_t).powf(SCHOLAR_SCAN_DISTANCE_EXPONENT);
    lerp(SCHOLAR_SCAN_GAIN_MIN, SCHOLAR_SCAN_GAIN_MAX, close_factor)
}

fn scholar_in_lock_target_volume(
    player: &HudPlayer,
    target_pos: HudPosition,
    target_distance: f32,
) -> bool {
    let radius = scholar_lock_target_radius()
        .mul_add(SCHOLAR_LOCK_TARGET_RADIUS_SCALE, 0.0)
        .clamp(8.0, SCHOLAR_SCAN_MAX_DISTANCE);
    if target_distance > radius {
        return false;
    }

    let player_pos = hud_physics_position(&player.actor);
    let dy = target_pos.1 - player_pos.1;
    if dy.abs() > SCHOLAR_LOCK_TARGET_MAX_VERTICAL {
        return false;
    }

    let Ok(camera) = native_camera::HudCamera::observe() else {
        return true;
    };
    let camera = camera.pers_cam_1.as_ref();
    let camera_pos = camera.position();
    let camera_to_target = [
        target_pos.0 - camera_pos.0,
        target_pos.1 - camera_pos.1,
        target_pos.2 - camera_pos.2,
    ];
    let right = [camera.matrix.0.0, camera.matrix.0.1, camera.matrix.0.2];
    let forward = [camera.matrix.2.0, camera.matrix.2.1, camera.matrix.2.2];
    let Some((_, target_depth)) = camera_depths_on_same_projection_side(
        [camera_pos.0, camera_pos.1, camera_pos.2],
        forward,
        [player_pos.0, player_pos.1, player_pos.2],
        [target_pos.0, target_pos.1, target_pos.2],
    ) else {
        return false;
    };

    let side = dot3(camera_to_target, right).abs();
    let side_limit_by_radius = (radius * SCHOLAR_LOCK_TARGET_MAX_SIDE_FACTOR)
        .max(10.0)
        .min(radius);
    let side_limit_by_depth = (target_depth.abs() + SCHOLAR_LOCK_TARGET_FORWARD_PAD)
        * SCHOLAR_LOCK_TARGET_SIDE_DEPTH_FACTOR;
    side <= side_limit_by_radius.min(side_limit_by_depth)
}

fn scholar_lock_target_radius() -> f32 {
    use shikifsrs_elden_ring::parameter_fields::{ParameterField as F, ScalarValue as V};
    let param_id = sdk_host::with_objects(|objects, epoch| {
        Ok(objects.game_camera_state(epoch)?.map(|v| {
            if v.parameter_ids[3] > 0 {
                v.parameter_ids[3]
            } else if v.parameter_ids[1] > 0 {
                v.parameter_ids[1]
            } else {
                v.parameter_ids[0]
            }
        }))
    })
    .ok()
    .flatten()
    .unwrap_or(0);
    if param_id <= 0 {
        return SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS;
    }
    [
        F::CameraLockRadius,
        F::CameraLockRadiusDistance,
        F::CameraLockRadiusPlayerDistance,
    ]
    .into_iter()
    .filter_map(
        |field| match native_parameters::read_field(field, param_id as u32) {
            Ok(Some(V::F32(value))) if value.is_finite() && value > 0. => Some(value),
            _ => None,
        },
    )
    .fold(SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS, f32::max)
}

fn distance3(a: native_camera::Vector4, b: native_camera::Vector4) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn distance_havok(a: HudPosition, b: HudPosition) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn havok_position_is_finite(position: &HudPosition) -> bool {
    position.0.is_finite() && position.1.is_finite() && position.2.is_finite()
}

fn dot_from_target(previous: HudPosition, current: HudPosition, target: HudPosition) -> f32 {
    let previous_to_target = target - previous;
    let current_to_target = target - current;
    previous_to_target.0 * current_to_target.0
        + previous_to_target.1 * current_to_target.1
        + previous_to_target.2 * current_to_target.2
}

fn scholar_chr_physics_position(target: &HudActor) -> HudPosition {
    hud_physics_position(target)
}

fn scholar_target_havok_position(target: &HudActor, height_factor: f32) -> Option<HudPosition> {
    let physics = native_character::physics_shape(target.identity.address).ok()?;
    let [x, y, z, _] = physics.position_bits.map(f32::from_bits);
    let lift = (physics.height * height_factor)
        .clamp(RECLUSE_MARKER_MIN_LOCK_LIFT, RECLUSE_MARKER_MAX_LOCK_LIFT);
    Some(HudPosition(x, y + lift, z, 0.0))
}

fn hud_cast_ray(
    player: &HudPlayer,
    filter: u32,
    origin: HudPosition,
    delta: HudDelta,
) -> shikifsrs_runtime::Result<Option<HudPosition>> {
    let target = native_vitals::identity(player.actor.identity.address)?;
    sdk_host::cast_line(
        target,
        filter,
        [origin.0, origin.1, origin.2],
        [delta.0, delta.1, delta.2],
    )
    .map(|hit| hit.map(|[x, y, z]| HudPosition(x, y, z, 0.0)))
}

fn scholar_line_of_sight_to_point(
    origin: HudPosition,
    target_pos: HudPosition,
    target: &HudActor,
    player: &HudPlayer,
) -> bool {
    let target_distance = distance_havok(origin, target_pos);
    let delta = HudDelta(
        target_pos.0 - origin.0,
        target_pos.1 - origin.1,
        target_pos.2 - origin.2,
    );
    let Ok(physics) = native_character::physics_shape(target.identity.address) else {
        return false;
    };
    let tolerance = (physics.radius * SCHOLAR_LINE_OF_SIGHT_RADIUS_TOLERANCE
        + physics.height * SCHOLAR_LINE_OF_SIGHT_HEIGHT_TOLERANCE)
        .max(SCHOLAR_LINE_OF_SIGHT_MIN_TOLERANCE);
    for filter in SCHOLAR_LINE_OF_SIGHT_FILTERS {
        let hit = match hud_cast_ray(player, filter, origin, delta) {
            Ok(Some(hit)) => hit,
            Ok(None) => continue,
            Err(error) => {
                eprintln!(
                    "ShikiFSRS HUD line query failed or uncertain; receipt retained: {error}"
                );
                return false;
            }
        };

        let hit_distance = distance_havok(origin, hit);
        if hit_distance <= SCHOLAR_LINE_OF_SIGHT_NEAR_HIT_IGNORE {
            continue;
        }
        if hit_distance + tolerance < target_distance {
            return false;
        }
    }

    true
}

fn scholar_has_line_of_sight(target: &HudActor, player: &HudPlayer) -> bool {
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return true;
    };
    let camera_origin = camera.pers_cam_1.as_ref().position();
    let origin = HudPosition(
        camera_origin.0,
        camera_origin.1,
        camera_origin.2,
        camera_origin.3,
    );
    [0.38, 0.58, 0.78].into_iter().any(|height_factor| {
        let Some(target_pos) = scholar_target_havok_position(target, height_factor) else {
            return false;
        };
        scholar_line_of_sight_to_point(origin, target_pos, target, player)
    })
}

fn resurrection_accumulation_teammate_marker_position(
    teammate: &HudActor,
) -> Option<native_camera::Vector4> {
    let offset = native_character::marker_geometry(teammate.identity).ok()?.frontend_displacement;
    let physics = native_character::physics_shape(teammate.identity.address).ok()?;
    let [x, y, z, w] = physics.position_bits.map(f32::from_bits);
    let base = HudPosition(x, y, z, w);
    let scale = native_character::axis_scale(teammate.identity).ok()?[1].abs().clamp(0.5, 4.0);
    let hit_height = if physics.height.is_finite() && physics.height > 0.1 {
        physics.height
    } else {
        RESURRECTION_TEAMMATE_FALLBACK_HEAD_LIFT * scale
    };
    let lift = (hit_height + RESURRECTION_TEAMMATE_HEAD_PADDING * scale).clamp(
        RECLUSE_MARKER_MIN_LOCK_LIFT,
        RESURRECTION_TEAMMATE_MAX_HEAD_LIFT,
    );

    Some(native_camera::Vector4(
        base.0 + offset[0],
        base.1 + lift,
        base.2 + offset[2],
        1.0,
    ))
}

fn scholar_is_event_hidden(chr: &HudActor) -> bool {
    let Ok(flags) = native_character::flags(chr.identity.address) else {
        // An unknown native presentation source cannot admit this target.
        return true;
    };
    let Ok(entry) = native_character::entry_state(chr.identity) else {
        return true;
    };
    flags.native_death_marked()
        || (entry.native_state_bytes[2] & 0x01) != 0
        || runtime_chr_force_unloaded(chr)
        || runtime_chr_character_disabled(chr)
        || (!native_character::load_requests(chr.identity.address)
            .is_ok_and(|value| value.draw_group_requested())
            && (!flags.model_enable_requested() || !flags.group_mask_result()))
}

fn scholar_is_lockable(chr: &HudActor) -> bool {
    let target = match native_vitals::identity(chr.identity.address) {
        Ok(target) => target,
        Err(error) => {
            eprintln!("ShikiFSRS HUD lock-target identity rejected: {error}");
            return false;
        }
    };
    match native_character::lock_target_suppression(target) {
        Ok(suppressed) => !suppressed,
        Err(error) => {
            eprintln!("ShikiFSRS HUD lock-target policy rejected: {error}");
            false
        }
    }
}

fn scholar_is_excluded_model(chr: &HudActor) -> bool {
    if chr.resource_number == 1000 || chr.character_number == 1000 {
        return true;
    }
    match native_character::initializer_override(chr.identity) {
        Ok(id) => id == 1000,
        Err(error) => {
            eprintln!("ShikiFSRS HUD initializer exclusion observation rejected: {error}");
            true
        }
    }
}

fn scholar_reject_reason(chr: &HudActor) -> Option<ScholarRejectReason> {
    let kind = match native_vitals::identity(chr.identity.address)
        .and_then(native_character::kind_record)
    {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("ShikiFSRS HUD target kind-record rejected: {error}");
            return Some(ScholarRejectReason::Type);
        }
    };
    // Byte 1 is the actual native classification-table predicate. Do not freeze its
    // writable initializers or infer the network role of the remaining table bytes.
    // Raw 19 retains the experimental consumer policy documented by the adapter.
    if kind.selected_bytes[0] != 0 || matches!(kind.selection.raw_kind, -1 | 0 | 1 | 17 | 19) {
        return Some(ScholarRejectReason::Type);
    }

    if scholar_is_event_hidden(chr) || !scholar_is_lockable(chr) || scholar_is_excluded_model(chr) {
        return Some(ScholarRejectReason::Hidden);
    }

    let data = match native_vitals::observe(chr.identity.address) {
        Ok(value) => value.hp,
        Err(error) => {
            eprintln!("ShikiFSRS HUD target vital copy rejected: {error}");
            return Some(ScholarRejectReason::Hp);
        }
    };
    if data.current <= 0 || data.maximum <= 0 {
        return Some(ScholarRejectReason::Hp);
    }

    None
}

fn scholar_progress_stage(progress: f32) -> Option<usize> {
    if progress >= 1.0 {
        Some(3)
    } else if progress >= 0.5 {
        Some(2)
    } else if progress > 0.0 {
        Some(1)
    } else {
        None
    }
}

fn valid_screen_pos(pos: [f32; 2]) -> Option<[f32; 2]> {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return None;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    (pos[0].is_finite()
        && pos[1].is_finite()
        && pos[0] >= -width * 0.25
        && pos[0] <= width * 1.25
        && pos[1] >= -height * 0.25
        && pos[1] <= height * 1.25)
        .then_some(pos)
}

fn screen_distance_sq(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

fn project_with_camera_axes(
    rel: [f32; 3],
    axes: ([f32; 3], [f32; 3], [f32; 3]),
    width: f32,
    height: f32,
    aspect: f32,
    tan_half_fov: f32,
) -> Option<[f32; 2]> {
    let (right, up, forward) = axes;
    let x = dot3(rel, right);
    let y = dot3(rel, up);
    let z_forward = dot3(rel, forward);
    if z_forward.abs() < 0.001 {
        return None;
    }

    let z = z_forward.abs();
    let signed_x = if z_forward < 0.0 { -x } else { x };
    let signed_y = if z_forward < 0.0 { -y } else { y };
    let ndc_x = signed_x / (z * tan_half_fov * aspect);
    let ndc_y = signed_y / (z * tan_half_fov);
    if !ndc_x.is_finite() || !ndc_y.is_finite() {
        return None;
    }

    let screen = [(ndc_x * 0.5 + 0.5) * width, (0.5 - ndc_y * 0.5) * height];
    (screen[0] >= -width * 0.25
        && screen[0] <= width * 1.25
        && screen[1] >= -height * 0.25
        && screen[1] <= height * 1.25)
        .then_some(screen)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn camera_depths_on_same_projection_side(
    camera_pos: [f32; 3],
    forward: [f32; 3],
    reference_pos: [f32; 3],
    target_pos: [f32; 3],
) -> Option<(f32, f32)> {
    let camera_to_reference = [
        reference_pos[0] - camera_pos[0],
        reference_pos[1] - camera_pos[1],
        reference_pos[2] - camera_pos[2],
    ];
    let camera_to_target = [
        target_pos[0] - camera_pos[0],
        target_pos[1] - camera_pos[1],
        target_pos[2] - camera_pos[2],
    ];
    let reference_depth = dot3(camera_to_reference, forward);
    let target_depth = dot3(camera_to_target, forward);
    (reference_depth.is_finite()
        && target_depth.is_finite()
        && reference_depth.abs() >= SCHOLAR_LOCK_TARGET_MIN_CAMERA_DEPTH
        && target_depth.abs() >= SCHOLAR_LOCK_TARGET_MIN_CAMERA_DEPTH
        && reference_depth.signum() == target_depth.signum())
    .then_some((reference_depth, target_depth))
}

#[cfg(test)]
mod camera_depth_side_tests {
    use super::camera_depths_on_same_projection_side;

    const CAMERA: [f32; 3] = [0.0, 0.0, 0.0];
    const FORWARD: [f32; 3] = [0.0, 0.0, 1.0];

    #[test]
    fn accepts_targets_on_the_players_camera_depth_side() {
        assert_eq!(
            camera_depths_on_same_projection_side(
                CAMERA,
                FORWARD,
                [0.0, 0.0, 3.0],
                [2.0, 0.0, 8.0],
            ),
            Some((3.0, 8.0))
        );
        assert_eq!(
            camera_depths_on_same_projection_side(
                CAMERA,
                FORWARD,
                [0.0, 0.0, -3.0],
                [2.0, 0.0, -8.0],
            ),
            Some((-3.0, -8.0))
        );
    }

    #[test]
    fn rejects_targets_behind_the_camera_relative_to_the_player() {
        assert_eq!(
            camera_depths_on_same_projection_side(
                CAMERA,
                FORWARD,
                [0.0, 0.0, 3.0],
                [0.0, 0.0, -8.0],
            ),
            None
        );
    }

    #[test]
    fn rejects_near_plane_and_non_finite_depths() {
        assert_eq!(
            camera_depths_on_same_projection_side(
                CAMERA,
                FORWARD,
                [0.0, 0.0, 3.0],
                [0.0, 0.0, 0.5],
            ),
            None
        );
        assert_eq!(
            camera_depths_on_same_projection_side(
                CAMERA,
                FORWARD,
                [0.0, 0.0, 3.0],
                [0.0, 0.0, f32::NAN],
            ),
            None
        );
    }

    #[test]
    fn scholar_filter_does_not_depend_on_a_fixed_animation_queue_slot() {
        let source = include_str!("damage.rs");
        let filter_start = source
            .find("fn scholar_reject_reason")
            .expect("Scholar target filter must exist");
        let filter_end = source[filter_start..]
            .find("\nfn scholar_progress_stage")
            .map(|offset| filter_start + offset)
            .expect("Scholar target filter must have a bounded body");
        let filter = &source[filter_start..filter_end];

        assert!(!filter.contains("scholar_current_anim_id"));
        assert!(!filter.contains("anim_queue"));
    }
}

#[cfg(test)]
mod undertaker_auto_dodge_tests {
    use super::{
        AtomicU64, Ordering, SP_UNDERTAKER_AUTO_DODGE_TRIGGER, UNDERTAKER_PENDING_UNKNOWN_ATTACK,
        UndertakerHitDispatchAction, capture_undertaker_pending_attack,
        find_unique_pattern_in_range, pack_undertaker_pending_attack, undertaker_dodge_effects,
        undertaker_hit_dispatch_action, undertaker_inflated_hit_radius,
        unpack_undertaker_pending_attack, valid_undertaker_hit_radius,
    };

    #[test]
    fn incoming_atk_key_uses_native_offsets_and_keeps_no_payload_pointer() {
        let mut payload = [0u8; 0x48];
        payload[0x40..0x44].copy_from_slice(&2110460i32.to_le_bytes());
        payload[0x44..0x48].copy_from_slice(&1i32.to_le_bytes());
        let pending = capture_undertaker_pending_attack(payload.as_ptr() as usize);
        payload.fill(0xff);
        assert_eq!(
            unpack_undertaker_pending_attack(pending),
            Some((1, 2110460))
        );
        assert_eq!(
            capture_undertaker_pending_attack(payload.as_ptr() as usize),
            UNDERTAKER_PENDING_UNKNOWN_ATTACK,
        );
        assert_eq!(
            capture_undertaker_pending_attack(0),
            UNDERTAKER_PENDING_UNKNOWN_ATTACK
        );
    }

    #[test]
    fn pending_atk_keys_validate_signed_fields_and_preserve_both_tables() {
        for table in 0..=1 {
            for row_id in [0, 2110460, i32::MAX] {
                assert_eq!(
                    unpack_undertaker_pending_attack(pack_undertaker_pending_attack(row_id, table)),
                    Some((table as u32, row_id as u32)),
                );
            }
        }
        for (row_id, table) in [(-1, 0), (i32::MIN, 1), (100, -1), (100, 2)] {
            let pending = pack_undertaker_pending_attack(row_id, table);
            assert_eq!(pending, UNDERTAKER_PENDING_UNKNOWN_ATTACK);
            assert_eq!(unpack_undertaker_pending_attack(pending), None);
        }
        assert_eq!(unpack_undertaker_pending_attack(u64::MAX), None);
    }

    #[test]
    fn only_five_directions_emit_a_marker_before_the_generic_request() {
        for direction in i8::MIN..=i8::MAX {
            let expected = match direction {
                0 => Some(880930),
                1 => Some(880931),
                2 => Some(880932),
                3 => Some(880933),
                4 => Some(880934),
                _ => None,
            };
            assert_eq!(
                undertaker_dodge_effects(pack_undertaker_pending_attack(100, 0), |_, _| Some(
                    direction
                )),
                [expected, Some(SP_UNDERTAKER_AUTO_DODGE_TRIGGER)],
            );
        }
    }

    #[test]
    fn same_atk_id_in_npc_and_pc_tables_uses_the_exact_source_table() {
        for (source, expected) in [(0, 880933), (1, 880934)] {
            let mut calls = 0;
            let effects = undertaker_dodge_effects(
                pack_undertaker_pending_attack(100, source),
                |table, id| {
                    calls += 1;
                    match (table, id) {
                        (0, 100) => Some(3),
                        (1, 100) => Some(4),
                        _ => None,
                    }
                },
            );
            assert_eq!(calls, 1);
            assert_eq!(effects, [Some(expected), Some(880922)]);
        }
    }

    #[test]
    fn unknown_or_missing_attack_keeps_only_generic_and_empty_queue_does_nothing() {
        assert_eq!(
            undertaker_dodge_effects(0, |_, _| panic!("empty queue lookup")),
            [None, None]
        );
        assert_eq!(
            undertaker_dodge_effects(UNDERTAKER_PENDING_UNKNOWN_ATTACK, |_, _| panic!(
                "unknown key lookup"
            )),
            [None, Some(880922)],
        );
        assert_eq!(
            undertaker_dodge_effects(pack_undertaker_pending_attack(100, 0), |_, _| None),
            [None, Some(880922)],
        );
    }

    #[test]
    fn first_pending_contact_is_atomic_and_consumption_or_clear_drops_its_key() {
        let pending = AtomicU64::new(0);
        let first = pack_undertaker_pending_attack(100, 0);
        let second = pack_undertaker_pending_attack(200, 1);
        assert_eq!(
            pending.compare_exchange(0, first, Ordering::AcqRel, Ordering::Acquire),
            Ok(0)
        );
        assert_eq!(
            pending.compare_exchange(0, second, Ordering::AcqRel, Ordering::Acquire),
            Err(first)
        );
        assert_eq!(pending.swap(0, Ordering::AcqRel), first);
        assert_eq!(pending.swap(0, Ordering::AcqRel), 0);
        assert_eq!(
            pending.compare_exchange(0, second, Ordering::AcqRel, Ordering::Acquire),
            Ok(0)
        );
        pending.store(0, Ordering::Release);
        assert_eq!(pending.swap(0, Ordering::AcqRel), 0);
    }

    #[test]
    fn only_local_positive_sensor_contacts_trigger() {
        let trigger =
            undertaker_hit_dispatch_action(0x1000, 0x1000, true, true, true, false, 100, 0, 0);
        assert_eq!(trigger, UndertakerHitDispatchAction::Trigger);

        for action in [
            undertaker_hit_dispatch_action(0x2000, 0x1000, true, true, true, false, 100, 0, 0),
            undertaker_hit_dispatch_action(0x1000, 0x1000, false, true, true, false, 100, 0, 0),
            undertaker_hit_dispatch_action(0x1000, 0x1000, true, false, true, false, 100, 0, 0),
            undertaker_hit_dispatch_action(0x1000, 0x1000, true, true, false, false, 100, 0, 0),
            undertaker_hit_dispatch_action(0x1000, 0x1000, true, true, true, false, 100, 0, 200),
        ] {
            assert_eq!(action, UndertakerHitDispatchAction::Forward);
        }
    }

    #[test]
    fn pending_and_bounded_suppression_reject_duplicate_local_hits() {
        assert_eq!(
            undertaker_hit_dispatch_action(0x1000, 0x1000, false, false, false, true, 100, 0, 0,),
            UndertakerHitDispatchAction::Reject,
        );
        assert_eq!(
            undertaker_hit_dispatch_action(0x1000, 0x1000, false, false, false, false, 499, 500, 0,),
            UndertakerHitDispatchAction::Reject,
        );
        assert_eq!(
            undertaker_hit_dispatch_action(0x1000, 0x1000, false, false, false, false, 500, 500, 0,),
            UndertakerHitDispatchAction::Forward,
        );
        assert_eq!(
            undertaker_hit_dispatch_action(0x2000, 0x1000, true, true, true, true, 100, 500, 0,),
            UndertakerHitDispatchAction::Forward,
        );
    }

    #[test]
    fn hurtbox_radius_is_validated_and_inflated_with_a_hard_cap() {
        assert!(valid_undertaker_hit_radius(0.4));
        assert!(!valid_undertaker_hit_radius(0.0));
        assert!(!valid_undertaker_hit_radius(f32::NAN));
        assert_eq!(undertaker_inflated_hit_radius(0.4), 0.8);
        assert_eq!(undertaker_inflated_hit_radius(3.8), 4.0);
    }

    #[test]
    fn native_patterns_must_match_exactly_once() {
        let pattern = [Some(0xAA), None, Some(0xCC)];
        let unique = [0x00, 0xAA, 0xBB, 0xCC, 0x00];
        assert_eq!(
            find_unique_pattern_in_range(&unique, 0, unique.len(), &pattern),
            Some(1),
        );

        let duplicate = [0xAA, 0x01, 0xCC, 0xAA, 0x02, 0xCC];
        assert_eq!(
            find_unique_pattern_in_range(&duplicate, 0, duplicate.len(), &pattern),
            None,
        );
        assert_eq!(
            find_unique_pattern_in_range(&unique, 2, unique.len(), &pattern),
            None,
        );
    }
}

fn update_local_damage_module(player: &mut HudPlayer) {
    LOCAL_DAMAGE_MODULE.store(damage_module_from_chr_ins(&player.actor), Ordering::Relaxed);
}

fn append_damage_log(_line: &str) {
    // Intentionally disabled for normal builds; keep diagnostics counters alive without writing files.
}
