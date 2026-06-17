static INSTALL_DAMAGE_HOOK: Once = Once::new();
static INSTALL_REVENANT_SUMMON_RANGE_HOOK: Once = Once::new();
static INSTALL_CAMERA_DIRECTED_POSITION_HOOK: Once = Once::new();
static INSTALL_MULTI_LOCK_BULLET_HOOK: Once = Once::new();
static MULTI_LOCK_BULLET_TARGET_CURSOR: AtomicUsize = AtomicUsize::new(0);
static MULTI_LOCK_BULLET_SPAWNING_CLONE: AtomicBool = AtomicBool::new(false);
static MULTI_LOCK_LIVE_BULLET_SEEN: LazyLock<Mutex<HashSet<FieldInsHandle>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));
static MULTI_LOCK_LIVE_BULLET_COUNT: AtomicU32 = AtomicU32::new(0);
static MULTI_LOCK_LIVE_GUIDED_BULLET_COUNT: AtomicU32 = AtomicU32::new(0);
static MULTI_LOCK_LIVE_WEAPON_GUIDANCE: LazyLock<
    Mutex<HashMap<FieldInsHandle, MultiLockLiveWeaponGuidance>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));
static MULTI_LOCK_SPAWN_HOOK_BULLET_PARAMS: LazyLock<Mutex<HashMap<i32, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static MULTI_LOCK_TARGET_MARKERS: LazyLock<Mutex<HashMap<FieldInsHandle, i32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ULTIMATE_DAMAGE_EVENTS: AtomicU32 = AtomicU32::new(0);
static LOCAL_DAMAGE_MODULE: AtomicUsize = AtomicUsize::new(0);
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

const RVA_DAMAGE_PROCESS: usize = 0x4483B0;
const REVENANT_SUMMON_RANGE_PATTERN: [Option<u8>; 15] = [
    Some(0x48),
    Some(0x8B),
    Some(0x47),
    None,
    Some(0xF3),
    Some(0x0F),
    Some(0x10),
    Some(0x90),
    None,
    None,
    None,
    None,
    Some(0x0F),
    Some(0x2F),
    Some(0xD0),
];
const REVENANT_SUMMON_RANGE_COMISS_OFFSET: usize = 12;
const REVENANT_SUMMON_RANGE_BUDDY_TYPE_ID: u32 = 0x7D0;
const REVENANT_SUMMON_RANGE_OVERRIDE: f32 = 1000.0;
const CAMERA_DIRECTED_POSITION_PATTERN: [Option<u8>; 11] = [
    Some(0x0F),
    Some(0x11),
    Some(0x43),
    Some(0x70),
    Some(0xC7),
    Some(0x43),
    Some(0x7C),
    Some(0x00),
    Some(0x00),
    Some(0x80),
    Some(0x3F),
];
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
const RVA_CS_BULLET_MANAGER_SPAWN_BULLET: usize = 0x3A2CA0;
const BULLET_SPAWN_OWNER_OFFSET: usize = 0x0;
const BULLET_SPAWN_BULLET_ID_OFFSET: usize = 0x14;
const BULLET_SPAWN_TARGET_OFFSET: usize = 0x20;
const CHR_MODULE_BAG_OFFSET: usize = 0x190;
const MODULE_RESIST_OFFSET: usize = 0x20;
const MODULE_DAMAGE_OFFSET: usize = 0x98;
const DAMAGE_VALUE_OFFSETS: [usize; 5] = [0x230, 0x234, 0x238, 0x23C, 0x240];
const DAMAGE_SCAN_START: usize = 0x200;
const DAMAGE_SCAN_END: usize = 0x280;
const DAMAGE_LOG_INTERVAL: f32 = 1.0;
const DUCHESS_REPLAY_HISTORY_SECONDS: f32 = 3.5;
const DUCHESS_REPLAY_HISTORY_CAP: usize = 512;
const CHR_PHYSICS_PROXY_OFFSET: usize = 0x98;
const CHR_PHYSICS_PROXY2_OFFSET: usize = 0xA0;
const CHR_PHYSICS_COLLISION_SHAPE_OFFSET: usize = 0xB0;
const CHR_INS_DEBUG_HASH_LEN: usize = 0x580;
const CHR_DATA_DEBUG_HASH_LEN: usize = 0x260;
const CHR_MSB_PARTS_DEBUG_HASH_LEN: usize = 0x50;
const CHR_SET_ENTRY_DEBUG_HASH_LEN: usize = 0x10;
const RECLUSE_LOCK_DEBUG_POINTS: bool = false;
const RECLUSE_MARKER_HIT_HEIGHT_FACTOR: f32 = 0.55;
const RECLUSE_MARKER_AABB_HEIGHT_FACTOR: f32 = 0.58;
const RECLUSE_MARKER_FALLBACK_BODY_LIFT: f32 = 1.35;
const RECLUSE_MARKER_MIN_LOCK_LIFT: f32 = 0.35;
const RECLUSE_MARKER_MAX_LOCK_LIFT: f32 = 18.0;
type LockDebugPoints = [Option<[f32; 2]>; 5];

#[derive(Clone, Copy)]
struct MultiLockLiveWeaponGuidance {
    target: FieldInsHandle,
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
        let hook_site = Program::current().image().as_ptr() as usize + RVA_DAMAGE_PROCESS;
        let hook = unsafe {
            ilhook::x64::hook_closure_jmp_back(
                hook_site,
                |regs: *mut ilhook::x64::Registers| {
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
                },
                ilhook::x64::CallbackOption::None,
                ilhook::x64::HookFlags::empty(),
            )
        };

        match hook {
            Ok(hook) => {
                append_damage_log(&format!("[init] damage hook installed at 0x{hook_site:X}"));
                std::mem::forget(hook);
            }
            Err(error) => {
                append_damage_log(&format!(
                    "[init] damage hook failed at 0x{hook_site:X}: {error:?}"
                ));
            }
        }
    });
}

fn install_revenant_summon_range_hook_once() {
    INSTALL_REVENANT_SUMMON_RANGE_HOOK.call_once(|| {
        let program = Program::current();
        let Some(pattern_rva) = find_revenant_summon_range_pattern(program) else {
            append_damage_log("[init] revenant summon range hook failed: pattern not found");
            return;
        };
        let hook_site = program.image().as_ptr() as usize
            + pattern_rva
            + REVENANT_SUMMON_RANGE_COMISS_OFFSET;
        let hook = unsafe {
            ilhook::x64::hook_closure_jmp_back(
                hook_site,
                |regs: *mut ilhook::x64::Registers| {
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
                },
                ilhook::x64::CallbackOption::None,
                ilhook::x64::HookFlags::empty(),
            )
        };

        match hook {
            Ok(hook) => {
                append_damage_log(&format!(
                    "[init] revenant summon range hook installed at 0x{hook_site:X}"
                ));
                std::mem::forget(hook);
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
        let program = Program::current();
        let Some(pattern_rva) = find_camera_directed_position_pattern(program) else {
            append_damage_log("[init] camera directed position hook failed: pattern not found");
            return;
        };
        let hook_site = program.image().as_ptr() as usize + pattern_rva;
        let hook = unsafe {
            ilhook::x64::hook_closure_jmp_back(
                hook_site,
                |regs: *mut ilhook::x64::Registers| {
                    apply_camera_directed_position_hook(regs);
                },
                ilhook::x64::CallbackOption::None,
                ilhook::x64::HookFlags::empty(),
            )
        };

        match hook {
            Ok(hook) => {
                append_damage_log(&format!(
                    "[init] camera directed position hook installed at 0x{hook_site:X}"
                ));
                std::mem::forget(hook);
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
        let hook_site =
            Program::current().image().as_ptr() as usize + RVA_CS_BULLET_MANAGER_SPAWN_BULLET;
        let hook = unsafe {
            ilhook::x64::hook_closure_jmp_back(
                hook_site,
                |regs: *mut ilhook::x64::Registers| {
                    apply_multi_lock_bullet_spawn_hook(regs);
                },
                ilhook::x64::CallbackOption::None,
                ilhook::x64::HookFlags::empty(),
            )
        };

        match hook {
            Ok(hook) => {
                append_damage_log(&format!(
                    "[init] multi lock bullet hook installed at 0x{hook_site:X}"
                ));
                std::mem::forget(hook);
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

fn find_revenant_summon_range_pattern(program: Program) -> Option<usize> {
    let image = program.image();
    let code_range = program.headers().code_range();
    let start = code_range.start as usize;
    let end = (code_range.end as usize).min(image.len());
    find_pattern_in_range(image, start, end, &REVENANT_SUMMON_RANGE_PATTERN)
}

fn find_camera_directed_position_pattern(program: Program) -> Option<usize> {
    let image = program.image();
    let code_range = program.headers().code_range();
    let start = code_range.start as usize;
    let end = (code_range.end as usize).min(image.len());
    find_pattern_in_range(image, start, end, &CAMERA_DIRECTED_POSITION_PATTERN)
}

fn find_pattern_in_range(
    image: &[u8],
    start: usize,
    end: usize,
    pattern: &[Option<u8>],
) -> Option<usize> {
    if pattern.is_empty() || end <= start || end - start < pattern.len() {
        return None;
    }
    image[start..end]
        .windows(pattern.len())
        .position(|window| {
            window
                .iter()
                .zip(pattern.iter())
                .all(|(byte, expected)| expected.map_or(true, |expected| *byte == expected))
        })
        .map(|offset| start + offset)
}

fn apply_multi_lock_bullet_spawn_hook(regs: *mut ilhook::x64::Registers) {
    if MULTI_LOCK_BULLET_SPAWNING_CLONE.load(Ordering::Relaxed) {
        return;
    }

    let manager = unsafe { (*regs).rcx as *mut CSBulletManager };
    let spawn_data = unsafe { (*regs).r8 as *mut BulletSpawnData };
    if manager.is_null() || spawn_data.is_null() {
        return;
    }

    let bullet_id = unsafe {
        read_i32((spawn_data as usize) + BULLET_SPAWN_BULLET_ID_OFFSET)
    };
    if bullet_id < 0 {
        return;
    }

    let owner = unsafe { read_field_ins_handle((spawn_data as usize) + BULLET_SPAWN_OWNER_OFFSET) };
    let original_target =
        unsafe { read_field_ins_handle((spawn_data as usize) + BULLET_SPAWN_TARGET_OFFSET) };
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
        unsafe {
            write_field_ins_handle((spawn_data as usize) + BULLET_SPAWN_TARGET_OFFSET, target);
        }
    }

    if target_plan.spawn_extra_clones {
        spawn_multi_lock_extra_bullets(manager, spawn_data, &targets, target);
    }
}

struct MultiLockBulletTargetPlan {
    targets: Vec<FieldInsHandle>,
    spawn_extra_clones: bool,
}

#[derive(Clone, Copy)]
struct MultiLockConfig {
    target_limit: usize,
    marker_sp_effect: i32,
}

fn multi_lock_bullet_targets(
    owner: FieldInsHandle,
    original_target: FieldInsHandle,
    bullet_manager: *mut CSBulletManager,
) -> Option<MultiLockBulletTargetPlan> {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return None;
    };
    let player = world_chr_man.main_player.as_ref()?;
    let owner_is_player = owner == player.chr_ins.field_ins_handle;
    let owner_belongs_to_player = owner_is_player
        || multi_lock_owner_belongs_to_player(owner, player.chr_ins.field_ins_handle, bullet_manager);
    let weapon_behavior_fallback = !owner_belongs_to_player
        && multi_lock_weapon_behavior_fallback(world_chr_man, player.as_ref(), &owner, original_target);
    if !owner_belongs_to_player && !weapon_behavior_fallback {
        return None;
    }

    multi_lock_targets_for_player(world_chr_man, player.as_ref(), original_target).map(|targets| {
        MultiLockBulletTargetPlan {
            targets,
            spawn_extra_clones: owner_is_player || weapon_behavior_fallback,
        }
    })
}

fn multi_lock_weapon_behavior_fallback(
    world_chr_man: &WorldChrMan,
    player: &PlayerIns,
    owner: &FieldInsHandle,
    original_target: FieldInsHandle,
) -> bool {
    if owner.selector.field_ins_type() == Some(FieldInsType::Bullet) || original_target.is_empty() {
        return false;
    }

    if original_target == player.chr_ins.field_ins_handle {
        return false;
    }

    let Some(target) = world_chr_man.chr_ins_by_handle(&original_target) else {
        return false;
    };
    if scholar_reject_reason(target).is_some() {
        return false;
    }
    let Some(screen_pos) = scholar_target_screen_pos(target) else {
        return false;
    };
    if !multi_lock_screen_pos_in_lens(screen_pos) {
        return false;
    }
    let player_pos = player.chr_ins.modules.as_ref().physics.as_ref().position;
    let target_pos = scholar_chr_physics_position(target);
    let target_distance = distance_havok(player_pos, target_pos);
    scholar_in_lock_target_volume(player, target_pos, target_distance)
        && scholar_has_line_of_sight(target, player)
}

fn spawn_multi_lock_extra_bullets(
    manager: *mut CSBulletManager,
    spawn_data: *const BulletSpawnData,
    targets: &[FieldInsHandle],
    primary_target: FieldInsHandle,
) {
    if targets.len() <= 1 {
        return;
    }

    MULTI_LOCK_BULLET_SPAWNING_CLONE.store(true, Ordering::Relaxed);
    for target in targets
        .iter()
        .copied()
        .filter(|target| *target != primary_target)
        .take(MULTI_LOCK_BULLET_EXTRA_CLONES_PER_SPAWN)
    {
        let mut cloned_spawn_data = unsafe { clone_bullet_spawn_data(spawn_data) };
        unsafe {
            write_field_ins_handle(
                (&mut cloned_spawn_data as *mut BulletSpawnData as usize)
                    + BULLET_SPAWN_TARGET_OFFSET,
                target,
            );
        }
        let _ = unsafe { (&mut *manager).spawn_bullet(&cloned_spawn_data) };
    }
    MULTI_LOCK_BULLET_SPAWNING_CLONE.store(false, Ordering::Relaxed);
}

unsafe fn clone_bullet_spawn_data(spawn_data: *const BulletSpawnData) -> BulletSpawnData {
    let mut cloned = std::mem::MaybeUninit::<BulletSpawnData>::uninit();
    unsafe {
        std::ptr::copy_nonoverlapping(
            spawn_data as *const u8,
            cloned.as_mut_ptr() as *mut u8,
            std::mem::size_of::<BulletSpawnData>(),
        );
        cloned.assume_init()
    }
}

fn update_multi_lock_live_bullets() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        return;
    };
    let Some(player) = world_chr_man.main_player.as_ref() else {
        MULTI_LOCK_LIVE_BULLET_COUNT.store(0, Ordering::Relaxed);
        return;
    };
    if multi_lock_target_limit(&player.chr_ins).is_none() {
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
        multi_lock_targets_for_player(world_chr_man, player.as_ref(), player.locked_on_enemy)
            .unwrap_or_default();
    let Ok(bullet_manager) = (unsafe { CSBulletManager::instance_mut() }) else {
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

    for bullet in bullet_manager.bullets_mut() {
        let handle = bullet.field_ins_handle;
        current.insert(handle);
        let param_id = multi_lock_bullet_param_id(bullet);
        let is_new = !previous.contains(&handle);
        let can_assign_guidance = is_new || bullet.time_alive <= MULTI_LOCK_LIVE_WEAPON_ASSIGN_WINDOW_SECONDS;
        let guided_target = if can_assign_guidance
            && guidance
                .as_ref()
                .is_none_or(|guidance| !guidance.contains_key(&handle))
        {
            assign_multi_lock_live_weapon_bullet_target(
                player.chr_ins.field_ins_handle,
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
                                + Duration::from_secs_f32(
                                    MULTI_LOCK_LIVE_WEAPON_GUIDANCE_SECONDS,
                                ),
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

        if let Some((target, mix)) = guided_target {
            if apply_multi_lock_live_weapon_bullet_direction(world_chr_man, bullet, target, mix) {
                guided_count += 1;
            }
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
    player_handle: FieldInsHandle,
    targets: &[FieldInsHandle],
    bullet: &eldenring::cs::CSBulletIns,
    param_id: i32,
) -> Option<FieldInsHandle> {
    if !multi_lock_should_guide_live_bullet_param(param_id)
        || bullet.targeting_owner.owner_chr_handle != player_handle
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
    world_chr_man: &WorldChrMan,
    bullet: &mut eldenring::cs::CSBulletIns,
    target_handle: FieldInsHandle,
    mix: f32,
) -> bool {
    let mix = mix.clamp(0.0, 1.0);
    if mix <= 0.001 {
        return false;
    }

    let Some(target) = world_chr_man.chr_ins_by_handle(&target_handle) else {
        return false;
    };

    let target_pos = scholar_target_havok_position(target, 0.55);
    let bullet_pos = bullet.physics.position;
    let dx = target_pos.0 - bullet_pos.0;
    let dy = target_pos.1 - bullet_pos.1;
    let dz = target_pos.2 - bullet_pos.2;
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if !len.is_finite() || len < 0.001 {
        return false;
    }

    let velocity = bullet.physics.velocity;
    let speed =
        (velocity.0 * velocity.0 + velocity.1 * velocity.1 + velocity.2 * velocity.2).sqrt();
    let speed = if speed.is_finite() && speed > 0.1 {
        speed
    } else {
        24.0
    };

    let original_dir = if speed > 0.001 {
        (velocity.0 / speed, velocity.1 / speed, velocity.2 / speed)
    } else {
        (dx / len, dy / len, dz / len)
    };
    let target_dir = (dx / len, dy / len, dz / len);
    let mixed_x = original_dir.0 * (1.0 - mix) + target_dir.0 * mix;
    let mixed_y = original_dir.1 * (1.0 - mix) + target_dir.1 * mix;
    let mixed_z = original_dir.2 * (1.0 - mix) + target_dir.2 * mix;
    let mixed_len = (mixed_x * mixed_x + mixed_y * mixed_y + mixed_z * mixed_z).sqrt();
    if !mixed_len.is_finite() || mixed_len < 0.001 {
        return false;
    }

    bullet.physics.velocity = DirectionalVector(
        mixed_x / mixed_len * speed,
        mixed_y / mixed_len * speed,
        mixed_z / mixed_len * speed,
        velocity.3,
    );
    true
}

fn multi_lock_bullet_param_id(bullet: &eldenring::cs::CSBulletIns) -> i32 {
    let fly = bullet.fly_state.base.param.param_id;
    if fly >= 0 {
        return fly;
    }
    let wait = bullet.wait_state.base.param.param_id;
    if wait >= 0 {
        return wait;
    }
    bullet.exp_state.base.param.param_id
}

fn multi_lock_snapshot_targets() -> (bool, [MultiLockTargetSnapshot; MULTI_LOCK_BULLET_MAX_TARGETS])
{
    let mut snapshot = [MultiLockTargetSnapshot::default(); MULTI_LOCK_BULLET_MAX_TARGETS];
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return (false, snapshot);
    };
    let Some(player) = world_chr_man.main_player.as_ref() else {
        refresh_multi_lock_target_markers(&[], None);
        return (false, snapshot);
    };
    let Some(config) = multi_lock_config(&player.chr_ins) else {
        refresh_multi_lock_target_markers(&[], None);
        return (false, snapshot);
    };

    if let Some(targets) = multi_lock_targets_for_player(
        world_chr_man,
        player.as_ref(),
        player.locked_on_enemy,
    ) {
        refresh_multi_lock_target_markers(&targets, Some(config.marker_sp_effect));
        for (index, (slot, handle)) in snapshot.iter_mut().zip(targets.iter()).enumerate() {
            let Some(pos) = world_chr_man
                .chr_ins_by_handle(handle)
                .and_then(scholar_target_screen_pos)
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

fn refresh_multi_lock_target_markers(targets: &[FieldInsHandle], marker_sp_effect: Option<i32>) {
    let Ok(mut marker_state) = MULTI_LOCK_TARGET_MARKERS.lock() else {
        return;
    };
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };

    let current_targets: HashSet<FieldInsHandle> = targets.iter().copied().collect();
    let previous: Vec<(FieldInsHandle, i32)> = marker_state
        .iter()
        .map(|(handle, marker)| (*handle, *marker))
        .collect();

    for (handle, old_marker) in previous {
        let should_remove = marker_sp_effect != Some(old_marker) || !current_targets.contains(&handle);
        if should_remove {
            if let Some(chr) = world_chr_man.chr_ins_by_handle_mut(&handle) {
                chr.remove_speffect(old_marker);
            }
            marker_state.remove(&handle);
        }
    }

    let Some(marker_sp_effect) = marker_sp_effect else {
        return;
    };
    for handle in targets {
        if let Some(chr) = world_chr_man.chr_ins_by_handle_mut(handle) {
            chr.apply_speffect(marker_sp_effect, true);
            marker_state.insert(*handle, marker_sp_effect);
        }
    }
}

fn multi_lock_owner_belongs_to_player(
    owner: FieldInsHandle,
    player_handle: FieldInsHandle,
    bullet_manager: *mut CSBulletManager,
) -> bool {
    if owner == player_handle {
        return true;
    }
    if owner.selector.field_ins_type() != Some(FieldInsType::Bullet) || bullet_manager.is_null() {
        return false;
    }
    unsafe { (&mut *bullet_manager).bullet_ins_by_handle(&owner) }
        .is_some_and(|bullet| bullet.targeting_owner.owner_chr_handle == player_handle)
}

fn multi_lock_targets_for_player(
    world_chr_man: &WorldChrMan,
    player: &PlayerIns,
    original_target: FieldInsHandle,
) -> Option<Vec<FieldInsHandle>> {
    let config = multi_lock_config(&player.chr_ins)?;
    let target_limit = config.target_limit;

    let mut targets = Vec::with_capacity(target_limit);
    push_multi_lock_target(&mut targets, original_target, target_limit, world_chr_man);
    push_multi_lock_target(
        &mut targets,
        player.locked_on_enemy,
        target_limit,
        world_chr_man,
    );

    let player_pos = player.chr_ins.modules.as_ref().physics.as_ref().position;
    let mut scanned = 0usize;
    for entry in world_chr_man.chr_inses_by_distance.iter() {
        if targets.len() >= target_limit || scanned >= MULTI_LOCK_BULLET_SCAN_MAX_ENTRIES {
            break;
        }
        scanned += 1;

        let chr = unsafe { entry.chr_ins.as_ref() };
        if targets.contains(&chr.field_ins_handle) || scholar_reject_reason(chr).is_some() {
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
        targets.push(chr.field_ins_handle);
    }

    Some(targets)
}

fn multi_lock_target_limit(player: &ChrIns) -> Option<usize> {
    multi_lock_config(player).map(|config| config.target_limit)
}

fn multi_lock_config(player: &ChrIns) -> Option<MultiLockConfig> {
    player
        .special_effect
        .entries()
        .filter_map(|entry| multi_lock_config_from_sp_effect(entry.param_id))
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
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
        return true;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return true;
    }
    let center = [width * SCHOLAR_LENS_CENTER_X_FACTOR, height * 0.5];
    let radius = height.min(width) * SCHOLAR_LENS_RADIUS_FACTOR * MULTI_LOCK_SCREEN_LENS_RADIUS_SCALE;
    let dx = pos[0] - center[0];
    let dy = pos[1] - center[1];
    dx * dx + dy * dy <= radius * radius
}

fn push_multi_lock_target(
    targets: &mut Vec<FieldInsHandle>,
    target: FieldInsHandle,
    target_limit: usize,
    world_chr_man: &WorldChrMan,
) {
    if targets.len() >= target_limit || target.is_empty() || targets.contains(&target) {
        return;
    }
    let Some(chr) = world_chr_man.chr_ins_by_handle(&target) else {
        return;
    };
    if scholar_reject_reason(chr).is_none() {
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
        || horizontal_len < CAMERA_DIRECTED_MIN_FRAME_DELTA
        || horizontal_len > CAMERA_DIRECTED_MAX_FRAME_DELTA
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

    let Some((camera_x, camera_z)) = camera_horizontal_direction() else {
        return None;
    };
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
        *bias = (*bias + delta * CAMERA_DIRECTED_VERTICAL_BIAS_RESPONSE)
            .clamp(CAMERA_DIRECTED_VERTICAL_BIAS_MIN, CAMERA_DIRECTED_VERTICAL_BIAS_MAX);
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
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
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
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return None;
    };
    let player = world_chr_man.main_player.as_ref()?;
    if !chr_has_speffect(&player.chr_ins, CAMERA_DIRECTED_MOVEMENT_EFFECT) {
        return None;
    }
    Some(player.chr_ins.modules.as_ref().physics.as_ref() as *const _ as usize)
}

unsafe fn read_f32(address: usize) -> f32 {
    unsafe { std::ptr::read_unaligned(address as *const f32) }
}

unsafe fn read_i32(address: usize) -> i32 {
    unsafe { std::ptr::read_unaligned(address as *const i32) }
}

unsafe fn read_field_ins_handle(address: usize) -> FieldInsHandle {
    unsafe { std::ptr::read_unaligned(address as *const FieldInsHandle) }
}

unsafe fn write_field_ins_handle(address: usize, handle: FieldInsHandle) {
    unsafe { std::ptr::write_unaligned(address as *mut FieldInsHandle, handle) }
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

fn duchess_replay_payload_for(
    target_damage_module: usize,
    now: Instant,
) -> DuchessReplayPayload {
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

    for (remaining_status, ignored_status) in remaining.status.iter_mut().zip(ignored.status.iter_mut()) {
        let consumed = (*remaining_status).min(*ignored_status);
        *remaining_status -= consumed;
        *ignored_status -= consumed;
    }

    let consumed_super_armor = remaining
        .super_armor_damage
        .min(ignored.super_armor_damage);
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

fn damage_module_from_chr_ins(chr_ins: &eldenring::cs::ChrIns) -> usize {
    let chr_ins = chr_ins as *const _ as usize;
    let module_bag = unsafe { *((chr_ins + CHR_MODULE_BAG_OFFSET) as *const usize) };
    if module_bag == 0 {
        0
    } else {
        unsafe { *((module_bag + MODULE_DAMAGE_OFFSET) as *const usize) }
    }
}

fn target_recluse_damage_module(
    world_chr_man: &WorldChrMan,
    locked_on_enemy: &FieldInsHandle,
) -> Option<usize> {
    if locked_on_enemy.is_empty() {
        return None;
    }

    let target = world_chr_man.chr_ins_by_handle(locked_on_enemy)?;
    let damage_module = damage_module_from_chr_ins(target);
    (damage_module != 0).then_some(damage_module)
}

fn locked_target_screen_pos(
    world_chr_man: &WorldChrMan,
    locked_on_enemy: &FieldInsHandle,
) -> Option<[f32; 2]> {
    if locked_on_enemy.is_empty() {
        return None;
    }

    let projected = locked_target_body_screen_pos(world_chr_man, locked_on_enemy);

    projected.or_else(|| locked_target_tag_screen_pos(locked_on_enemy))
}

fn locked_target_body_screen_pos(
    world_chr_man: &WorldChrMan,
    locked_on_enemy: &FieldInsHandle,
) -> Option<[f32; 2]> {
    let target = world_chr_man.chr_ins_by_handle(locked_on_enemy)?;
    let camera = unsafe { CSCamera::instance() }.ok()?;
    let body_position = recluse_body_marker_position(target);
    project_lock_on_position(body_position, camera.pers_cam_1.as_ref())
}

fn recluse_body_marker_position(target: &eldenring::cs::ChrIns) -> fromsoftware_shared::F32Vector4 {
    let ctrl = target.chr_ctrl.as_ref();
    let base = ctrl.model_matrix.3;
    let tag_offset = ctrl.lock_on_chr_tag_dmypoly_offset;
    let lift = recluse_body_marker_lift(target, base.1);

    fromsoftware_shared::F32Vector4(
        base.0 + tag_offset.0,
        base.1 + lift,
        base.2 + tag_offset.2,
        1.0,
    )
}

fn recluse_body_marker_lift(target: &eldenring::cs::ChrIns, base_y: f32) -> f32 {
    let ctrl = target.chr_ctrl.as_ref();
    let physics = target.modules.as_ref().physics.as_ref();
    let hit_lift = physics.hit_height * RECLUSE_MARKER_HIT_HEIGHT_FACTOR;
    let aabb = target
        .chr_model_ins
        .as_ref()
        .model_ins
        .model_item
        .location_aabb_exporter
        .as_ref()
        .aabb;
    let aabb_lift =
        (aabb.min.1 + (aabb.max.1 - aabb.min.1) * RECLUSE_MARKER_AABB_HEIGHT_FACTOR) - base_y;

    match (
        valid_recluse_marker_lift(hit_lift),
        valid_recluse_marker_lift(aabb_lift),
    ) {
        (true, true) => return hit_lift.max(aabb_lift),
        (true, false) => return hit_lift,
        (false, true) => return aabb_lift,
        (false, false) => {}
    }

    let lock_lift = target.lock_on_target_position.1 - base_y;
    if valid_recluse_marker_lift(lock_lift) {
        return lock_lift;
    }

    RECLUSE_MARKER_FALLBACK_BODY_LIFT * ctrl.scale_size_y.abs().clamp(0.5, 4.0)
}

fn valid_recluse_marker_lift(lift: f32) -> bool {
    lift.is_finite()
        && (RECLUSE_MARKER_MIN_LOCK_LIFT..=RECLUSE_MARKER_MAX_LOCK_LIFT).contains(&lift)
}

fn project_lock_on_position(
    position: fromsoftware_shared::F32Vector4,
    camera: &eldenring::cs::CSPersCam,
) -> Option<[f32; 2]> {
    project_lock_on_position_with_mode(position, camera, true)
}

fn project_lock_on_position_column(
    position: fromsoftware_shared::F32Vector4,
    camera: &eldenring::cs::CSPersCam,
) -> Option<[f32; 2]> {
    project_lock_on_position_with_mode(position, camera, false)
}

fn project_lock_on_position_with_mode(
    position: fromsoftware_shared::F32Vector4,
    camera: &eldenring::cs::CSPersCam,
    use_row_axes: bool,
) -> Option<[f32; 2]> {
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
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

fn locked_target_tag_screen_pos(locked_on_enemy: &FieldInsHandle) -> Option<[f32; 2]> {
    locked_target_enemy_display_screen_pos(locked_on_enemy)
        .or_else(|| locked_target_frontend_tag_screen_pos(locked_on_enemy))
}

fn locked_target_enemy_display_screen_pos(locked_on_enemy: &FieldInsHandle) -> Option<[f32; 2]> {
    let fe_man = unsafe { CSFeManImp::instance() }.ok()?;
    if let Some(tag) = fe_man.enemy_chr_tag_displays.iter().find(|tag| {
        tag.field_ins_handle == *locked_on_enemy
            && tag.is_visible
            && tag.last_update_time_delta <= 0.25
    }) {
        return valid_screen_pos([tag.screen_pos.0, tag.screen_pos.1]);
    }

    None
}

fn locked_target_frontend_tag_screen_pos(locked_on_enemy: &FieldInsHandle) -> Option<[f32; 2]> {
    let fe_man = unsafe { CSFeManImp::instance() }.ok()?;
    fe_man
        .frontend_values
        .enemy_chr_tag_data
        .iter()
        .find(|tag| {
            tag.field_ins_handle == *locked_on_enemy
                && tag.is_visible
                && tag.update_position
                && !tag.is_not_on_screen
        })
        .and_then(|tag| valid_screen_pos([tag.screen_pos_x as f32, tag.screen_pos_y as f32]))
}

fn locked_target_debug_screen_points(
    world_chr_man: &WorldChrMan,
    locked_on_enemy: &FieldInsHandle,
) -> LockDebugPoints {
    if locked_on_enemy.is_empty() {
        return [None; 5];
    }

    let Some(target) = world_chr_man.chr_ins_by_handle(locked_on_enemy) else {
        return [None; 5];
    };
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return [None; 5];
    };
    let camera = camera.pers_cam_1.as_ref();
    let ctrl = target.chr_ctrl.as_ref();
    let tag_position = fromsoftware_shared::F32Vector4(
        ctrl.model_matrix.3.0 + ctrl.lock_on_chr_tag_dmypoly_offset.0,
        ctrl.model_matrix.3.1 + ctrl.lock_on_chr_tag_dmypoly_offset.1,
        ctrl.model_matrix.3.2 + ctrl.lock_on_chr_tag_dmypoly_offset.2,
        1.0,
    );

    [
        project_lock_on_position(target.lock_on_target_position, camera),
        project_lock_on_position_column(target.lock_on_target_position, camera),
        project_lock_on_position(tag_position, camera),
        locked_target_enemy_display_screen_pos(locked_on_enemy),
        locked_target_frontend_tag_screen_pos(locked_on_enemy),
    ]
}

fn scholar_target_screen_pos(target: &eldenring::cs::ChrIns) -> Option<[f32; 2]> {
    let camera = unsafe { CSCamera::instance() }.ok()?;
    let body_position = scholar_body_marker_position(target);
    project_lock_on_position(body_position, camera.pers_cam_1.as_ref())
}

fn scholar_body_marker_position(target: &ChrIns) -> fromsoftware_shared::F32Vector4 {
    let ctrl = target.chr_ctrl.as_ref();
    let tag_offset = ctrl.lock_on_chr_tag_dmypoly_offset;
    let physics = target.modules.as_ref().physics.as_ref();
    let base = physics.position;
    let lift = (physics.hit_height * RECLUSE_MARKER_HIT_HEIGHT_FACTOR)
        .clamp(RECLUSE_MARKER_MIN_LOCK_LIFT, RECLUSE_MARKER_MAX_LOCK_LIFT);

    fromsoftware_shared::F32Vector4(
        base.0 + tag_offset.0,
        base.1 + lift,
        base.2 + tag_offset.2,
        1.0,
    )
}

fn scholar_screen_pos_in_lens(pos: [f32; 2]) -> bool {
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
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

fn scholar_screen_pos_lens_score(pos: [f32; 2]) -> Option<f32> {
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
        return None;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let center = [width * SCHOLAR_LENS_CENTER_X_FACTOR, height * 0.5];
    Some(screen_distance_sq(pos, center))
}

fn scholar_screen_pos_on_view(pos: [f32; 2]) -> bool {
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
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
    player: &PlayerIns,
    target_pos: HavokPosition,
    target_distance: f32,
) -> bool {
    let radius = scholar_lock_target_radius()
        .mul_add(SCHOLAR_LOCK_TARGET_RADIUS_SCALE, 0.0)
        .clamp(8.0, SCHOLAR_SCAN_MAX_DISTANCE);
    if target_distance > radius {
        return false;
    }

    let player_pos = player.chr_ins.modules.as_ref().physics.as_ref().position;
    let dy = target_pos.1 - player_pos.1;
    if dy.abs() > SCHOLAR_LOCK_TARGET_MAX_VERTICAL {
        return false;
    }

    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return true;
    };
    let camera = camera.pers_cam_1.as_ref();
    let camera_pos = camera.position();
    let camera_to_player = [
        player_pos.0 - camera_pos.0,
        player_pos.1 - camera_pos.1,
        player_pos.2 - camera_pos.2,
    ];
    let camera_to_target = [
        target_pos.0 - camera_pos.0,
        target_pos.1 - camera_pos.1,
        target_pos.2 - camera_pos.2,
    ];
    let right = [camera.matrix.0.0, camera.matrix.0.1, camera.matrix.0.2];
    let forward = [camera.matrix.2.0, camera.matrix.2.1, camera.matrix.2.2];
    let player_depth = dot3(camera_to_player, forward);
    let target_depth = dot3(camera_to_target, forward);
    if !player_depth.is_finite()
        || !target_depth.is_finite()
        || player_depth.abs() < SCHOLAR_LOCK_TARGET_MIN_CAMERA_DEPTH
        || target_depth.abs() < SCHOLAR_LOCK_TARGET_MIN_CAMERA_DEPTH
        || player_depth.signum() != target_depth.signum()
    {
        return false;
    }

    let side = dot3(camera_to_target, right).abs();
    let side_limit_by_radius = (radius * SCHOLAR_LOCK_TARGET_MAX_SIDE_FACTOR)
        .max(10.0)
        .min(radius);
    let side_limit_by_depth =
        (target_depth.abs() + SCHOLAR_LOCK_TARGET_FORWARD_PAD) * SCHOLAR_LOCK_TARGET_SIDE_DEPTH_FACTOR;
    side <= side_limit_by_radius.min(side_limit_by_depth)
}

fn scholar_lock_target_radius() -> f32 {
    let param_id = (unsafe { GameMan::instance() })
        .ok()
        .map(|game_man| {
            if game_man.lock_on_camera_param_id > 0 {
                game_man.lock_on_camera_param_id
            } else if game_man.locked_camera_param_id > 0 {
                game_man.locked_camera_param_id
            } else {
                game_man.normal_camera_param_id
            }
        })
        .unwrap_or(0);
    if param_id <= 0 {
        return SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS;
    }

    let Ok(param_repository) = (unsafe { SoloParamRepository::instance() }) else {
        return SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS;
    };
    let Some(lock_cam) = param_repository.get::<LockCamParam>(param_id as u32) else {
        return SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS;
    };

    [
        lock_cam.chr_lock_range_max_radius(),
        lock_cam.chr_lock_range_max_radius_for_d(),
        lock_cam.chr_lock_range_max_radius_for_pd(),
    ]
    .into_iter()
    .filter(|value| value.is_finite() && *value > 0.0)
    .fold(SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS, f32::max)
}

fn distance3(a: fromsoftware_shared::F32Vector4, b: fromsoftware_shared::F32Vector4) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn distance_havok(a: HavokPosition, b: HavokPosition) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn scholar_chr_physics_position(target: &ChrIns) -> HavokPosition {
    target.modules.as_ref().physics.as_ref().position
}

fn scholar_target_havok_position(target: &ChrIns, height_factor: f32) -> HavokPosition {
    let physics = target.modules.as_ref().physics.as_ref();
    let lift = (physics.hit_height * height_factor)
        .clamp(RECLUSE_MARKER_MIN_LOCK_LIFT, RECLUSE_MARKER_MAX_LOCK_LIFT);
    HavokPosition(
        physics.position.0,
        physics.position.1 + lift,
        physics.position.2,
        0.0,
    )
}

fn scholar_line_of_sight_to_point(
    havok: &eldenring::cs::CSHavokMan,
    origin: HavokPosition,
    target_pos: HavokPosition,
    target: &ChrIns,
    player: &PlayerIns,
) -> bool {
    let target_distance = distance_havok(origin, target_pos);
    let delta = PositionDelta(
        target_pos.0 - origin.0,
        target_pos.1 - origin.1,
        target_pos.2 - origin.2,
    );
    let physics = target.modules.as_ref().physics.as_ref();
    let tolerance = (physics.hit_radius * SCHOLAR_LINE_OF_SIGHT_RADIUS_TOLERANCE
        + physics.hit_height * SCHOLAR_LINE_OF_SIGHT_HEIGHT_TOLERANCE)
        .max(SCHOLAR_LINE_OF_SIGHT_MIN_TOLERANCE);
    for filter in SCHOLAR_LINE_OF_SIGHT_FILTERS {
        let Some(hit) = havok
            .phys_world
            .as_ref()
            .cast_ray(filter, &origin, delta, player)
        else {
            continue;
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

fn scholar_has_line_of_sight(target: &ChrIns, player: &PlayerIns) -> bool {
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return true;
    };
    let Ok(havok) = (unsafe { CSHavokMan::instance() }) else {
        return true;
    };

    let origin = camera.pers_cam_1.as_ref().position();
    [0.38, 0.58, 0.78].into_iter().any(|height_factor| {
        scholar_line_of_sight_to_point(
            havok,
            origin,
            scholar_target_havok_position(target, height_factor),
            target,
            player,
        )
    })
}

fn scholar_candidate_debug(
    chr: &eldenring::cs::ChrIns,
    distance: f32,
    player: &PlayerIns,
) -> ScholarCandidateDebug {
    let data = chr.modules.as_ref().data.as_ref();
    let ctrl = chr.chr_ctrl.as_ref();
    let physics = chr.modules.as_ref().physics.as_ref();
    let chr_set_entry = unsafe { chr.chr_set_entry.as_ref() };
    let load_status = chr_set_entry.chr_load_status as u8;
    let (los_target_distance, los_hit_mask, los_near_mask, los_block_mask, los_closest_hit_distance, los_block_filter) =
        scholar_los_filter_debug(chr, player);
    ScholarCandidateDebug {
        active: true,
        selector_index: chr.field_ins_handle.selector.index(),
        selector_container: chr.field_ins_handle.selector.container(),
        block_area: chr.field_ins_handle.block_id.area(),
        block_block: chr.field_ins_handle.block_id.block(),
        block_region: chr.field_ins_handle.block_id.region(),
        block_index: chr.field_ins_handle.block_id.index(),
        npc_id: chr.npc_id,
        chr_type: chr.chr_type as i32,
        hp: data.hp,
        max_hp: data.max_hp,
        distance,
        load_status,
        chr_update_type: chr_set_entry.chr_update_type as u8,
        draw_group: chr.load_state.draw_group_enabled(),
        backread_disabled: chr.load_state.backread_disabled(),
        host_inactive: chr.load_state.host_inactive(),
        extinction_death: chr.load_state.extinction_death(),
        near_pc: chr.load_state.near_pc(),
        render_group: chr.chr_flags1c4.is_render_group_enabled(),
        onscreen_flag: chr.chr_flags1c4.is_onscreen(),
        enable_render: chr.chr_flags1c5.enable_render(),
        death_flag: chr.chr_flags1c5.death_flag(),
        is_active: chr.chr_flags1c8.is_active(),
        update_tasks_registered: chr.chr_flags1c8.update_tasks_registered(),
        force_unloaded: chr.debug_flags.force_unloaded(),
        character_disabled: chr.debug_flags.character_disabled(),
        tint_alpha: chr.tint_alpha_multiplier,
        tint_alpha_modifier: chr.tint_alpha_multiplier_modifier,
        base_transparency: chr.base_transparency,
        base_transparency_modifier: chr.base_transparency_modifier,
        event_entity_id: chr.event_entity_id,
        chr_set_entry_flags: chr_set_entry.entry_flags,
        activation_enabled: chr.chr_activation_flags.activation_enabled(),
        activate_threshold_exceeded: chr.chr_flags1ca.activate_threshold_exceeded(),
        sounds_active: chr.chr_flags1ca.sounds_active(),
        opacity_keyframes: chr.opacity_keyframes_multiplier,
        opacity_keyframes_previous: chr.opacity_keyframes_multiplier_previous,
        camouflage_transparency: chr.camouflage_transparency,
        distance_to_player_sqr: chr.distance_to_player_sqr,
        horizontal_distance_to_player_sqr: chr.horizontal_distance_to_player_sqr,
        max_render_range: chr.max_render_range,
        chr_activate_threshold: chr.chr_activate_threshold,
        current_anim_id: scholar_current_anim_id(chr),
        request_anim_id: chr.modules.as_ref().event.as_ref().request_animation_id,
        idle_anim_id: chr.modules.as_ref().event.as_ref().idle_anim_id,
        current_tae_id: chr
            .modules
            .as_ref()
            .action_request
            .as_ref()
            .action_request_queue
            .current_tae_id,
        chr_collision: ctrl.chr_collision,
        ctrl_disable_move: ctrl.disable_move,
        ctrl_disable_player_collision: ctrl.flags.disable_player_collision(),
        ctrl_disable_hit: ctrl.flags.disable_hit(),
        ctrl_disable_map_collision: ctrl.flags.disable_map_collision(),
        ctrl_disable_capsule_collision: ctrl.flags.disable_character_capsule_collision(),
        ctrl_disable_object_collision: ctrl.flags.disable_object_collision(),
        ctrl_proxy_pos_sync: ctrl.chr_proxy_flags.position_sync_requested(),
        ctrl_proxy_rot_sync: ctrl.chr_proxy_flags.rotation_sync_requested(),
        physics_chr_proxy: read_usize_field(physics, CHR_PHYSICS_PROXY_OFFSET),
        physics_chr_proxy2: read_usize_field(physics, CHR_PHYSICS_PROXY2_OFFSET),
        physics_collision_shape: read_usize_field(physics, CHR_PHYSICS_COLLISION_SHAPE_OFFSET),
        physics_pos_update_requested: physics.chr_proxy_pos_update_requested,
        physics_standing_on_ground: physics.standing_on_solid_ground,
        physics_touching_ground: physics.touching_solid_ground,
        physics_slide_enabled: physics.slide_info.enabled,
        physics_gravity_disabled: physics.gravity_disabled,
        chr_ptr: chr as *const eldenring::cs::ChrIns as usize,
        data_ptr: data as *const _ as usize,
        chr_set_entry_ptr: chr.chr_set_entry.as_ptr() as usize,
        msb_draw_flags: data.msb_parts.cs_msb_parts.draw_flags,
        msb_part_ptr: data.msb_parts.cs_msb_parts.msb_part.as_ptr() as usize,
        chr_hash: hash_bytes(chr, CHR_INS_DEBUG_HASH_LEN),
        data_hash: hash_bytes(data, CHR_DATA_DEBUG_HASH_LEN),
        msb_hash: hash_bytes(&data.msb_parts, CHR_MSB_PARTS_DEBUG_HASH_LEN),
        entry_hash: hash_bytes(chr_set_entry, CHR_SET_ENTRY_DEBUG_HASH_LEN),
        entry_raw0: read_u64_field(chr_set_entry, 0),
        entry_raw1: read_u64_field(chr_set_entry, 8),
        los_target_distance,
        los_hit_mask,
        los_near_mask,
        los_block_mask,
        los_closest_hit_distance,
        los_block_filter,
    }
}

fn read_usize_field<T>(base: &T, offset: usize) -> usize {
    unsafe { *((base as *const T as usize + offset) as *const usize) }
}

fn read_u64_field<T>(base: &T, offset: usize) -> u64 {
    unsafe { *((base as *const T as usize + offset) as *const u64) }
}

fn hash_bytes<T>(base: &T, len: usize) -> u32 {
    let bytes = unsafe { std::slice::from_raw_parts(base as *const T as *const u8, len) };
    let mut hash = 0x811C9DC5u32;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn scholar_los_filter_debug(target: &ChrIns, player: &PlayerIns) -> (f32, u32, u32, u32, f32, i32) {
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return (0.0, 0, 0, 0, -1.0, -1);
    };
    let Ok(havok) = (unsafe { CSHavokMan::instance() }) else {
        return (0.0, 0, 0, 0, -1.0, -1);
    };

    let origin = camera.pers_cam_1.as_ref().position();
    let target_pos = scholar_target_havok_position(target, 0.58);
    let target_distance = distance_havok(origin, target_pos);
    let delta = PositionDelta(
        target_pos.0 - origin.0,
        target_pos.1 - origin.1,
        target_pos.2 - origin.2,
    );
    let physics = target.modules.as_ref().physics.as_ref();
    let tolerance = (physics.hit_radius * SCHOLAR_LINE_OF_SIGHT_RADIUS_TOLERANCE
        + physics.hit_height * SCHOLAR_LINE_OF_SIGHT_HEIGHT_TOLERANCE)
        .max(SCHOLAR_LINE_OF_SIGHT_MIN_TOLERANCE);

    let mut hit_mask = 0;
    let mut near_mask = 0;
    let mut block_mask = 0;
    let mut closest_hit_distance = f32::MAX;
    let mut block_filter = -1;
    for (index, filter) in SCHOLAR_LINE_OF_SIGHT_FILTERS.into_iter().enumerate() {
        let Some(hit) = havok
            .phys_world
            .as_ref()
            .cast_ray(filter, &origin, delta, player)
        else {
            continue;
        };
        let bit = 1u32 << index;
        hit_mask |= bit;

        let hit_distance = distance_havok(origin, hit);
        if hit_distance < closest_hit_distance {
            closest_hit_distance = hit_distance;
        }
        if hit_distance <= SCHOLAR_LINE_OF_SIGHT_NEAR_HIT_IGNORE {
            near_mask |= bit;
            continue;
        }
        if hit_distance + tolerance < target_distance {
            block_mask |= bit;
            if block_filter < 0 {
                block_filter = filter as i32;
            }
        }
    }

    if closest_hit_distance == f32::MAX {
        closest_hit_distance = -1.0;
    }

    (
        target_distance,
        hit_mask,
        near_mask,
        block_mask,
        closest_hit_distance,
        block_filter,
    )
}

fn scholar_is_event_hidden(chr: &eldenring::cs::ChrIns) -> bool {
    let chr_set_entry = unsafe { chr.chr_set_entry.as_ref() };
    chr.chr_flags1c5.death_flag()
        || (chr_set_entry.entry_flags & 0x01) != 0
        || chr.debug_flags.force_unloaded()
        || chr.debug_flags.character_disabled()
        || (!chr.load_state.draw_group_enabled()
            && (!chr.chr_flags1c5.enable_render() || !chr.chr_flags1c4.is_render_group_enabled()))
}

fn scholar_is_lockable(chr: &eldenring::cs::ChrIns) -> bool {
    let modifier = chr.chr_ctrl.as_ref().modifier.as_ref();
    !modifier.data.action_flags.disable_lock_on()
}

fn scholar_is_excluded_model(chr: &eldenring::cs::ChrIns) -> bool {
    let data = chr.modules.as_ref().data.as_ref();
    chr.npc_id == 1000 || chr.character_id == 1000 || data.chara_init_param_id == 1000
}

fn scholar_current_anim_id(chr: &eldenring::cs::ChrIns) -> i32 {
    let time_act = chr.modules.as_ref().time_act.as_ref();
    time_act.anim_queue[2].anim_id
}

fn scholar_reject_reason(chr: &eldenring::cs::ChrIns) -> Option<ScholarRejectReason> {
    if matches!(
        chr.chr_type,
        ChrType::None
            | ChrType::Local
            | ChrType::WhitePhantom
            | ChrType::Ghost
            | ChrType::Ghost1
            | ChrType::BloodstainGhost
            | ChrType::BonfireGhost
            | ChrType::MessageGhost
            | ChrType::WhiteSummonNpc
            | ChrType::BluePhantom
    ) {
        return Some(ScholarRejectReason::Type);
    }

    if scholar_is_event_hidden(chr) || !scholar_is_lockable(chr) || scholar_is_excluded_model(chr)
    {
        return Some(ScholarRejectReason::Hidden);
    }

    if scholar_current_anim_id(chr) == -1 {
        return Some(ScholarRejectReason::Hidden);
    }

    let data = chr.modules.as_ref().data.as_ref();
    if data.hp <= 0 || data.max_hp <= 0 {
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
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
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

fn update_local_damage_module(player: &mut eldenring::cs::PlayerIns) {
    let chr_ins = &mut player.chr_ins as *mut _ as usize;
    let module_bag = unsafe { *((chr_ins + CHR_MODULE_BAG_OFFSET) as *const usize) };
    let damage_module = if module_bag == 0 {
        0
    } else {
        unsafe { *((module_bag + MODULE_DAMAGE_OFFSET) as *const usize) }
    };
    LOCAL_DAMAGE_MODULE.store(damage_module, Ordering::Relaxed);
}

fn append_damage_log(_line: &str) {
    // Intentionally disabled for normal builds; keep diagnostics counters alive without writing files.
}
