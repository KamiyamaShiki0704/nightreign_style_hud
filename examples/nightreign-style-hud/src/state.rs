static CAMERA_DIRECTED_LAST_POSITION: LazyLock<Mutex<Option<HudPosition>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Copy)]
struct HudSnapshot {
    role: Role,
    skill_layers: [f32; 2],
    skill_ready: bool,
    skill_top_down: bool,
    skill_count: Option<usize>,
    wylder_skill_lock_active: bool,
    wylder_skill_lock_has_target: bool,
    wylder_skill_lock_pos: Option<[f32; 2]>,
    wylder_skill_lock_scale: f32,
    ironeye_precision_aiming: bool,
    standalone_precision_aiming: bool,
    precision_calibration_aiming: bool,
    ironeye_weakness_marks: [IroneyeWeaknessSnapshot; IRONEYE_WEAKNESS_MAX_TARGETS],
    ironeye_weakness_bursts: [IroneyeWeaknessBurstSnapshot; IRONEYE_WEAKNESS_MAX_BURSTS],
    recluse_elements: [Option<RecluseElement>; 3],
    recluse_ready_magic: Option<usize>,
    recluse_lock_element: Option<RecluseElement>,
    recluse_lock_pos: Option<[f32; 2]>,
    recluse_lock_debug_points: LockDebugPoints,
    recluse_blood_debug: RecluseBloodDebug,
    scholar_observing: bool,
    scholar_targets: [ScholarTargetSnapshot; SCHOLAR_SCAN_MAX_TARGETS],
    scholar_damage_numbers: [ScholarDamageNumberSnapshot; SCHOLAR_DAMAGE_NUMBER_MAX],
    scholar_debug: ScholarScanDebug,
    revenant_summons: [RevenantSummonSnapshot; REVENANT_SUMMON_COUNT],
    revenant_skill_move_hud: bool,
    revenant_effect_mask: u8,
    revenant_active_summon: Option<usize>,
    revenant_buddy_count: usize,
    revenant_active_handle_container: u32,
    revenant_active_handle_index: u32,
    revenant_active_npc_id: i32,
    undertaker_normal_skill_buff: bool,
    undertaker_enhanced_skill_buff: bool,
    ultimate_progress: f32,
    ultimate_ready: bool,
    ultimate_top_down: bool,
    hud_solid: bool,
    undertaker_free_ultimate_active: bool,
    executor_ultimate_active: bool,
}

#[derive(Clone, Copy, Default)]
struct ResurrectionAccumulationSnapshot {
    active: bool,
    progress: f32,
}

#[derive(Clone, Copy, Default)]
struct TeammateResurrectionAccumulationSnapshot {
    progress: f32,
    screen_pos: [f32; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WylderHookshotNativeResult {
    PullEnemy,
    PullToEnemy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WylderSkillLockTargetKind {
    Enemy(HudHandle),
    Map,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WylderSkillLockTarget {
    kind: WylderSkillLockTargetKind,
    screen_pos: [f32; 2],
    world_pos: HudPosition,
    distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WylderHookshotMovementDestination {
    Enemy(HudHandle),
    Point(HudPosition),
}

impl WylderHookshotMovementDestination {
    fn stop_distance(self) -> f32 {
        match self {
            Self::Enemy(_) => WYLDER_HOOKSHOT_STOP_ENEMY_DISTANCE,
            Self::Point(_) => WYLDER_HOOKSHOT_STOP_MAP_DISTANCE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WylderHookshotCapsule {
    height: f32,
    radius: f32,
}

fn wylder_hookshot_capsule_from_values(
    chr_hit_height: f32,
    chr_hit_radius: f32,
    hit_height: f32,
    hit_radius: f32,
) -> Option<WylderHookshotCapsule> {
    let valid = |height: f32, radius: f32| {
        height.is_finite()
            && radius.is_finite()
            && (WYLDER_HOOKSHOT_CAPSULE_MIN_HEIGHT..=WYLDER_HOOKSHOT_CAPSULE_MAX_HEIGHT)
                .contains(&height)
            && (WYLDER_HOOKSHOT_CAPSULE_MIN_RADIUS..=WYLDER_HOOKSHOT_CAPSULE_MAX_RADIUS)
                .contains(&radius)
    };
    if valid(chr_hit_height, chr_hit_radius) {
        return Some(WylderHookshotCapsule {
            height: chr_hit_height,
            radius: chr_hit_radius,
        });
    }
    valid(hit_height, hit_radius).then_some(WylderHookshotCapsule {
        height: hit_height,
        radius: hit_radius,
    })
}

fn wylder_hookshot_chr_capsule(chr: &HudActor) -> Option<WylderHookshotCapsule> {
    let physics = native_character::physics(chr.identity.address).ok()?;
    let [character, map] = physics.collision_dimensions();
    wylder_hookshot_capsule_from_values(character[0], character[1], map[0], map[1])
}

fn wylder_hookshot_capsules_touch(
    player_position: HudPosition,
    player_capsule: WylderHookshotCapsule,
    target_position: HudPosition,
    target_capsule: WylderHookshotCapsule,
) -> bool {
    if !havok_position_is_finite(&player_position) || !havok_position_is_finite(&target_position) {
        return false;
    }

    let dx = player_position.0 - target_position.0;
    let dz = player_position.2 - target_position.2;
    let horizontal_limit =
        player_capsule.radius + target_capsule.radius + WYLDER_HOOKSHOT_STOP_CAPSULE_MARGIN;
    let vertical_limit = (player_capsule.height + target_capsule.height) * 0.5
        + WYLDER_HOOKSHOT_STOP_VERTICAL_MARGIN;
    dx * dx + dz * dz <= horizontal_limit * horizontal_limit
        && (player_position.1 - target_position.1).abs() <= vertical_limit
}

fn wylder_hookshot_close_enough(
    destination: WylderHookshotMovementDestination,
    player_position: HudPosition,
    target_position: HudPosition,
    player_capsule: Option<WylderHookshotCapsule>,
    target_capsule: Option<WylderHookshotCapsule>,
) -> bool {
    match (destination, player_capsule, target_capsule) {
        (
            WylderHookshotMovementDestination::Enemy(_),
            Some(player_capsule),
            Some(target_capsule),
        ) => wylder_hookshot_capsules_touch(
            player_position,
            player_capsule,
            target_position,
            target_capsule,
        ),
        _ => distance_havok(player_position, target_position) <= destination.stop_distance(),
    }
}

struct WylderHookshotMovementStopDetector {
    active: bool,
    destination: Option<WylderHookshotMovementDestination>,
    started_at: Option<Instant>,
    start_position: Option<HudPosition>,
    previous_position: Option<HudPosition>,
    initial_distance: f32,
    closest_distance: f32,
    emitted: bool,
}

impl Default for WylderHookshotMovementStopDetector {
    fn default() -> Self {
        Self {
            active: false,
            destination: None,
            started_at: None,
            start_position: None,
            previous_position: None,
            initial_distance: f32::INFINITY,
            closest_distance: f32::INFINITY,
            emitted: false,
        }
    }
}

impl WylderHookshotMovementStopDetector {
    fn begin(
        &mut self,
        destination: Option<WylderHookshotMovementDestination>,
        player_position: HudPosition,
        now: Instant,
    ) {
        *self = Self {
            active: true,
            destination,
            started_at: Some(now),
            start_position: Some(player_position),
            previous_position: Some(player_position),
            initial_distance: f32::INFINITY,
            closest_distance: f32::INFINITY,
            emitted: false,
        };
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn set_destination_if_missing(
        &mut self,
        destination: Option<WylderHookshotMovementDestination>,
        player_position: HudPosition,
        now: Instant,
    ) {
        if self.destination.is_some() || destination.is_none() {
            return;
        }
        self.destination = destination;
        self.started_at = Some(now);
        self.start_position = Some(player_position);
        self.previous_position = Some(player_position);
        self.initial_distance = f32::INFINITY;
        self.closest_distance = f32::INFINITY;
    }

    fn sample(
        &mut self,
        player_position: HudPosition,
        target_position: Option<HudPosition>,
        player_capsule: Option<WylderHookshotCapsule>,
        target_capsule: Option<WylderHookshotCapsule>,
        now: Instant,
    ) -> bool {
        if !self.active || self.emitted {
            return false;
        }
        let Some(destination) = self.destination else {
            return false;
        };
        let Some(target_position) = target_position.filter(havok_position_is_finite) else {
            return false;
        };
        if !havok_position_is_finite(&player_position) {
            return false;
        }

        let distance = distance_havok(player_position, target_position);
        if !distance.is_finite() {
            return false;
        }
        if !self.initial_distance.is_finite() {
            self.initial_distance = distance;
            self.closest_distance = distance;
        }

        let travelled = self
            .start_position
            .map(|start| distance_havok(start, player_position))
            .unwrap_or(0.0);
        let crossed_target_plane = self.previous_position.is_some_and(|previous| {
            dot_from_target(previous, player_position, target_position) <= 0.0
        });
        let approached = self.closest_distance
            <= self.initial_distance - WYLDER_HOOKSHOT_STOP_MIN_APPROACH_DISTANCE;
        let rebounded =
            approached && distance >= self.closest_distance + WYLDER_HOOKSHOT_STOP_REBOUND_DISTANCE;

        self.closest_distance = self.closest_distance.min(distance);
        self.previous_position = Some(player_position);

        let elapsed = self
            .started_at
            .map(|started| now.saturating_duration_since(started).as_secs_f32())
            .unwrap_or(0.0);
        let close_enough = wylder_hookshot_close_enough(
            destination,
            player_position,
            target_position,
            player_capsule,
            target_capsule,
        );
        let passed_destination = travelled >= WYLDER_HOOKSHOT_STOP_MIN_TRAVEL_DISTANCE
            && (crossed_target_plane || rebounded);
        if elapsed < WYLDER_HOOKSHOT_STOP_GRACE_SECONDS || (!close_enough && !passed_destination) {
            return false;
        }

        self.emitted = true;
        true
    }
}

#[derive(Default)]
struct WylderHookshotNativeResolver {
    active: bool,
    result: Option<WylderHookshotNativeResult>,
}

impl WylderHookshotNativeResolver {
    fn is_resolved(&self) -> bool {
        self.active && self.result.is_some()
    }

    fn update(
        &mut self,
        active: bool,
        enemy_result: Option<WylderHookshotNativeResult>,
    ) -> Option<WylderHookshotNativeResult> {
        if !active {
            *self = Self::default();
            return None;
        }
        if !self.active {
            self.active = true;
            self.result = None;
        }
        if self.result.is_some() {
            return None;
        }

        if let Some(result) = enemy_result {
            self.result = Some(result);
            return Some(result);
        }
        None
    }
}

struct ChargeState {
    role: Option<Role>,
    skill_layers: [f32; 2],
    ultimate: f32,
    last_update: Instant,
    last_skill_consume: Instant,
    last_recluse_absorb: Instant,
    last_hit_gain: Instant,
    last_kill_gain: Instant,
    last_critical_gain: Instant,
    last_damage_log: Instant,
    undertaker_buff_end: Option<Instant>,
    undertaker_buff_duration: f32,
    undertaker_auto_end_pending: bool,
    undertaker_free_ultimate_end: Option<Instant>,
    executor_transform_end: Option<Instant>,
    executor_auto_end_pending: bool,
    recluse_elements: [Option<RecluseElement>; 3],
    recluse_ready_magic: Option<usize>,
    recluse_lock_element: Option<RecluseElement>,
    recluse_lock_damage_module: Option<usize>,
    recluse_lock_pos: Option<[f32; 2]>,
    recluse_lock_debug_points: LockDebugPoints,
    ironeye_weakness_marks: HashMap<HudHandle, IroneyeWeaknessMark>,
    ironeye_weakness_bursts: Vec<IroneyeWeaknessBurst>,
    scholar_targets: HashMap<HudHandle, ScholarTargetState>,
    scholar_observing: bool,
    scholar_debug: ScholarScanDebug,
    scholar_sympathy_hp: HashMap<HudHandle, i32>,
    scholar_damage_numbers: Vec<ScholarDamageNumber>,
    scholar_native_damage_ignores: HashMap<HudHandle, (i32, Instant)>,
    scholar_enemy_effect_targets: Vec<HudHandle>,
    duchess_snapshots: HashMap<usize, DuchessReplaySnapshot>,
    recluse_blood_brands: HashMap<HudHandle, RecluseBloodBrand>,
    recluse_blood_pending_damage: Vec<RecluseBloodDamageEvent>,
    recluse_blood_debug: RecluseBloodDebug,
    recluse_blood_fixed_fp_accumulator: f32,
    recluse_outputs_clear_at: Option<Instant>,
    wylder_skill_lock_active: bool,
    wylder_skill_lock_has_target: bool,
    wylder_skill_lock_pos: Option<[f32; 2]>,
    wylder_skill_lock_scale: f32,
    wylder_hookshot_aim_target: Option<WylderSkillLockTarget>,
    wylder_hookshot_native: WylderHookshotNativeResolver,
    wylder_hookshot_target: Option<HudHandle>,
    wylder_hookshot_movement_stop: WylderHookshotMovementStopDetector,
    revenant_skill_move_hud: bool,
    revenant_active_summon: Option<usize>,
    revenant_effect_mask: u8,
    revenant_summon_hp: [f32; REVENANT_SUMMON_COUNT],
    revenant_summon_handles: [Option<HudHandle>; REVENANT_SUMMON_COUNT],
    revenant_summon_effect_seen: [bool; REVENANT_SUMMON_COUNT],
    revenant_summon_death_locked: [bool; REVENANT_SUMMON_COUNT],
    revenant_summon_restore_timer: [f32; REVENANT_SUMMON_COUNT],
    revenant_buddy_count: usize,
    revenant_known_buddy_handles: Vec<HudHandle>,
    revenant_bind_baseline: Vec<HudHandle>,
    revenant_pending_summon_bind: Option<usize>,
    revenant_map_refresh_cleanup_pending: bool,
    revenant_converted_souls: HashMap<u32, RevenantConvertedSoul>,
    revenant_debug_souls: Vec<RevenantDebugSoul>,
    revenant_pending_debug_soul: Option<RevenantPendingDebugSoul>,
    revenant_seen_deaths: HashSet<HudHandle>,
    resurrection_accumulation: ResurrectionAccumulationSnapshot,
    teammate_resurrection_accumulations: Vec<TeammateResurrectionAccumulationSnapshot>,
}

#[derive(Clone, Copy, Default)]
struct RevenantSummonSnapshot {
    active: bool,
    hp_ratio: f32,
}

#[derive(Clone, Copy, Default)]
struct DuchessReplaySnapshot {
    hp: i32,
    status: [i32; 7],
    super_armor: f32,
}

#[derive(Clone)]
struct RevenantConvertedSoul {
    expires_at: Instant,
    buddy_param_id: u32,
    handle: Option<HudHandle>,
}

#[derive(Clone, Copy)]
struct RevenantDebugSoul {
    expires_at: Instant,
    requested_at: Instant,
    handle: Option<HudHandle>,
    creation_request: native_creation::RequestId,
    // Retain creation provenance for unresolved native requests.
    #[allow(dead_code)]
    npc_id: i32,
    #[allow(dead_code)]
    npc_param_id: i32,
    hide_at: Option<Instant>,
}

#[derive(Clone, Copy)]
struct RevenantPendingDebugSoul {
    candidate: RevenantPassiveCandidate,
    ready_at: Instant,
}

impl DuchessReplaySnapshot {
    fn payload_since(self, previous: Self) -> DuchessReplayPayload {
        let mut status = [0; 7];
        for (index, slot) in status.iter_mut().enumerate() {
            let current = self.status[index];
            let previous = previous.status[index];
            *slot = (current - previous).max(0);
        }

        DuchessReplayPayload {
            hp_damage: (previous.hp - self.hp).max(0),
            status,
            super_armor_damage: (previous.super_armor - self.super_armor).max(0.0),
        }
    }
}

impl ChargeState {
    fn new(now: Instant) -> Self {
        Self {
            role: None,
            skill_layers: [0.0, 0.0],
            ultimate: 0.0,
            last_update: now,
            last_skill_consume: now - Duration::from_secs_f32(SKILL_CONSUME_INTERVAL),
            last_recluse_absorb: now - Duration::from_secs_f32(RECLUSE_ABSORB_INTERVAL),
            last_hit_gain: now - Duration::from_secs_f32(ULTIMATE_HIT_GAIN_INTERVAL),
            last_kill_gain: now - Duration::from_secs_f32(ULTIMATE_KILL_GAIN_INTERVAL),
            last_critical_gain: now - Duration::from_secs_f32(ULTIMATE_CRITICAL_GAIN_INTERVAL),
            last_damage_log: now,
            undertaker_buff_end: None,
            undertaker_buff_duration: UNDERTAKER_BUFF_SECONDS,
            undertaker_auto_end_pending: false,
            undertaker_free_ultimate_end: None,
            executor_transform_end: None,
            executor_auto_end_pending: false,
            recluse_elements: [None, None, None],
            recluse_ready_magic: None,
            recluse_lock_element: None,
            recluse_lock_damage_module: None,
            recluse_lock_pos: None,
            recluse_lock_debug_points: [None; 5],
            ironeye_weakness_marks: HashMap::new(),
            ironeye_weakness_bursts: Vec::new(),
            scholar_targets: HashMap::new(),
            scholar_observing: false,
            scholar_debug: ScholarScanDebug::default(),
            scholar_sympathy_hp: HashMap::new(),
            scholar_damage_numbers: Vec::new(),
            scholar_native_damage_ignores: HashMap::new(),
            scholar_enemy_effect_targets: Vec::new(),
            duchess_snapshots: HashMap::new(),
            recluse_blood_brands: HashMap::new(),
            recluse_blood_pending_damage: Vec::new(),
            recluse_blood_debug: RecluseBloodDebug::default(),
            recluse_blood_fixed_fp_accumulator: 0.0,
            recluse_outputs_clear_at: None,
            wylder_skill_lock_active: false,
            wylder_skill_lock_has_target: false,
            wylder_skill_lock_pos: None,
            wylder_skill_lock_scale: 1.0,
            wylder_hookshot_aim_target: None,
            wylder_hookshot_native: WylderHookshotNativeResolver::default(),
            wylder_hookshot_target: None,
            wylder_hookshot_movement_stop: WylderHookshotMovementStopDetector::default(),
            revenant_skill_move_hud: false,
            revenant_active_summon: None,
            revenant_effect_mask: 0,
            revenant_summon_hp: [1.0; REVENANT_SUMMON_COUNT],
            revenant_summon_handles: [None; REVENANT_SUMMON_COUNT],
            revenant_summon_effect_seen: [false; REVENANT_SUMMON_COUNT],
            revenant_summon_death_locked: [false; REVENANT_SUMMON_COUNT],
            revenant_summon_restore_timer: [0.0; REVENANT_SUMMON_COUNT],
            revenant_buddy_count: 0,
            revenant_known_buddy_handles: Vec::new(),
            revenant_bind_baseline: Vec::new(),
            revenant_pending_summon_bind: None,
            revenant_map_refresh_cleanup_pending: false,
            revenant_converted_souls: HashMap::new(),
            revenant_debug_souls: Vec::new(),
            revenant_pending_debug_soul: None,
            revenant_seen_deaths: HashSet::new(),
            resurrection_accumulation: ResurrectionAccumulationSnapshot::default(),
            teammate_resurrection_accumulations: Vec::new(),
        }
    }

    fn sync(&mut self, now: Instant) -> Option<HudSnapshot> {
        let delta = (now - self.last_update).as_secs_f32().clamp(0.0, 0.25);
        self.last_update = now;
        self.resurrection_accumulation = ResurrectionAccumulationSnapshot::default();
        self.teammate_resurrection_accumulations.clear();

        let mut copied_world = HudWorld::observe().ok()?;
        let world_chr_man = &mut copied_world;
        self.finish_revenant_map_refresh_cleanup(world_chr_man);
        let (
            active_effects,
            locked_on_enemy,
            action_pressed,
            super_armor_progress,
            player_handle,
            player_team_type,
            player_position,
        ) = {
            let player = world_chr_man.player.as_mut()?;
            update_local_damage_module(player);
            let active_effects = native_character::effects(player.actor.identity.address)
                .map_err(|error| eprintln!("ShikiFSRS HUD effects rejected: {error}"))
                .ok()?;
            let action_pressed = native_character::action_requested(player.actor.identity, 4)
                .map_err(|error| eprintln!("ShikiFSRS HUD interaction request rejected: {error}"))
                .ok()?;
            // SAFETY: This actual callback owns the experimental full posture/game-data graph;
            // native maximum querying is explicit and its actual receipt stays retained.
            let super_armor_progress =
                unsafe { native_vitals::super_armor_values(player.actor.identity.address) }
                    .map_or_else(
                        |error| {
                            eprintln!("ShikiFSRS HUD local posture query rejected: {error}");
                            0.0
                        },
                        |(current, maximum)| normalized_super_armor(current, maximum),
                    );
            (
                active_effects,
                player.locked_target,
                action_pressed,
                super_armor_progress,
                player.actor.handle,
                player.actor.team,
                hud_physics_position(&player.actor),
            )
        };
        self.resurrection_accumulation = ResurrectionAccumulationSnapshot {
            active: active_effects.contains(&SP_RESURRECTION_ACCUMULATION_HUD),
            progress: super_armor_progress,
        };
        collect_resurrection_accumulation_teammate_snapshots(
            &mut self.teammate_resurrection_accumulations,
            world_chr_man,
            player_handle,
            player_team_type,
            player_position,
        );

        let role = active_role_from_effects(&active_effects);
        let damage_hook_events = drain_damage_hook_events();
        self.update_recluse_blood_song(world_chr_man, now, &damage_hook_events);
        self.update_wylder_hookshot_native_resolution(world_chr_man, &active_effects);
        self.update_wylder_hookshot_movement_stop(
            world_chr_man,
            &active_effects,
            role == Some(Role::Wylder),
            now,
        );
        sync_precision_calibration_outputs(world_chr_man, &active_effects);
        set_revenant_summon_range_hook_active(role == Some(Role::Revenant));
        if role != self.role {
            if let Some(old_role) = self.role {
                let player = world_chr_man.player.as_mut()?;
                remove_role_outputs(player, role_config(old_role));
                if old_role == Role::Scholar {
                    self.clear_scholar_enemy_effects(world_chr_man, role_config(old_role));
                } else if old_role == Role::Ironeye {
                    self.clear_ironeye_weakness_marks(world_chr_man, role_config(old_role));
                } else if old_role == Role::Revenant {
                    self.clear_revenant_converted_souls(world_chr_man);
                }
            }
            self.reset_for_role(role);
        }

        let Some(role) = role else {
            set_undertaker_auto_dodge_armed(false);
            ULTIMATE_DAMAGE_EVENTS.store(0, Ordering::Relaxed);
            self.scholar_sympathy_hp.clear();
            self.scholar_damage_numbers.clear();
            self.scholar_native_damage_ignores.clear();
            self.scholar_enemy_effect_targets.clear();
            self.duchess_snapshots.clear();
            self.ironeye_weakness_marks.clear();
            self.ironeye_weakness_bursts.clear();
            let player = world_chr_man.player.as_mut()?;
            remove_all_role_outputs(player);
            remove_revenant_summon_death_effects_from_player(player);
            return None;
        };

        let config = role_config(role);
        if role == Role::Recluse {
            let current_damage_module =
                target_recluse_damage_module(world_chr_man, &locked_on_enemy);
            let current_element = current_damage_module.and_then(lookup_recluse_damage_element);
            if current_element.is_some() {
                self.recluse_lock_pos = locked_target_screen_pos(world_chr_man, &locked_on_enemy);
                self.recluse_lock_debug_points =
                    locked_target_debug_screen_points(world_chr_man, &locked_on_enemy);
                self.recluse_lock_damage_module = current_damage_module;
                self.recluse_lock_element = current_element;
            } else {
                self.recluse_lock_element = None;
                self.recluse_lock_damage_module = None;
                self.recluse_lock_pos = None;
                self.recluse_lock_debug_points = [None; 5];
            }
        } else {
            self.recluse_lock_element = None;
            self.recluse_lock_damage_module = None;
            self.recluse_lock_pos = None;
            self.recluse_lock_debug_points = [None; 5];
        }
        self.update_scholar_targets(world_chr_man, role, &active_effects, delta);
        self.update_wylder_skill_lock(world_chr_man, role, &active_effects);
        self.revenant_skill_move_hud =
            role == Role::Revenant && active_effects.contains(&SP_REVENANT_SKILL_MOVE_HUD);
        self.update_scholar_sympathy(world_chr_man, role, &active_effects, locked_on_enemy, now);
        self.update_ironeye_weakness(world_chr_man, role, now, &damage_hook_events);
        self.update_duchess_replay_history(world_chr_man, role);
        if role == Role::Revenant {
            patch_revenant_buddy_stones_once();
        }
        self.update_revenant_summons(world_chr_man, role, &active_effects, delta);
        self.update_revenant_passive(world_chr_man, role, now);
        if role == Role::Revenant {
            let revenant_buddy_handles = revenant_buddy_group_handles(world_chr_man);
            hide_revenant_native_summon_hud(&revenant_buddy_handles);
        }
        let mut copied_player = world_chr_man.player?;
        let player = &mut copied_player;
        update_local_damage_module(player);
        self.log_damage_diagnostics(now);
        self.advance_skill(config, now, delta);
        self.advance_ultimate(config, now, delta);
        self.apply_pending_undertaker_events(player, config);
        self.apply_pending_executor_events(player, config);
        self.clear_recluse_transient_outputs(player, config, now);
        self.consume_damage_events(now);
        self.consume_inputs(
            player,
            config,
            &active_effects,
            self.recluse_lock_element,
            self.recluse_lock_damage_module,
            world_chr_man,
            now,
        );
        self.sync_output_speffects(player, config);
        set_undertaker_auto_dodge_armed(
            config.role == Role::Undertaker
                && self.undertaker_buff_end.is_some()
                && self.undertaker_buff_duration >= UNDERTAKER_ENHANCED_BUFF_SECONDS
                && chr_has_speffect(&player.actor, SP_UNDERTAKER_ENHANCED_SKILL_BUFF),
        );

        Some(self.snapshot(world_chr_man, config, action_pressed, now))
    }

    fn reset_for_role(&mut self, role: Option<Role>) {
        self.role = role;
        self.skill_layers = [0.0, 0.0];
        self.ultimate = 0.0;
        self.last_skill_consume =
            self.last_update - Duration::from_secs_f32(SKILL_CONSUME_INTERVAL);
        self.last_recluse_absorb =
            self.last_update - Duration::from_secs_f32(RECLUSE_ABSORB_INTERVAL);
        self.last_hit_gain = self.last_update - Duration::from_secs_f32(ULTIMATE_HIT_GAIN_INTERVAL);
        self.last_kill_gain =
            self.last_update - Duration::from_secs_f32(ULTIMATE_KILL_GAIN_INTERVAL);
        self.last_critical_gain =
            self.last_update - Duration::from_secs_f32(ULTIMATE_CRITICAL_GAIN_INTERVAL);
        self.undertaker_buff_end = None;
        self.undertaker_buff_duration = UNDERTAKER_BUFF_SECONDS;
        self.undertaker_auto_end_pending = false;
        self.undertaker_free_ultimate_end = None;
        self.executor_transform_end = None;
        self.executor_auto_end_pending = false;
        self.recluse_elements = [None, None, None];
        self.recluse_ready_magic = None;
        self.recluse_lock_element = None;
        self.recluse_lock_damage_module = None;
        self.recluse_lock_pos = None;
        self.recluse_lock_debug_points = [None; 5];
        self.wylder_skill_lock_active = false;
        self.wylder_skill_lock_has_target = false;
        self.wylder_skill_lock_pos = None;
        self.wylder_skill_lock_scale = 1.0;
        self.wylder_hookshot_aim_target = None;
        self.wylder_hookshot_movement_stop.reset();
        self.revenant_skill_move_hud = false;
        self.scholar_targets.clear();
        self.scholar_observing = false;
        self.scholar_sympathy_hp.clear();
        self.scholar_damage_numbers.clear();
        self.scholar_native_damage_ignores.clear();
        self.scholar_enemy_effect_targets.clear();
        self.duchess_snapshots.clear();
        self.ironeye_weakness_marks.clear();
        self.ironeye_weakness_bursts.clear();
        self.recluse_outputs_clear_at = None;
        self.revenant_active_summon = None;
        self.revenant_effect_mask = 0;
        self.revenant_summon_hp = [1.0; REVENANT_SUMMON_COUNT];
        self.revenant_summon_handles = [None; REVENANT_SUMMON_COUNT];
        self.revenant_summon_effect_seen = [false; REVENANT_SUMMON_COUNT];
        self.revenant_summon_death_locked = [false; REVENANT_SUMMON_COUNT];
        self.revenant_summon_restore_timer = [0.0; REVENANT_SUMMON_COUNT];
        self.revenant_buddy_count = 0;
        self.revenant_known_buddy_handles.clear();
        self.revenant_bind_baseline.clear();
        self.revenant_pending_summon_bind = None;
        self.revenant_map_refresh_cleanup_pending = false;
        self.revenant_converted_souls.clear();
        self.revenant_debug_souls.clear();
        self.revenant_pending_debug_soul = None;
        self.revenant_seen_deaths.clear();
    }

    fn recluse_blood_debug(&self) -> RecluseBloodDebug {
        self.recluse_blood_debug
    }

    fn resurrection_accumulation_snapshot(&self) -> ResurrectionAccumulationSnapshot {
        self.resurrection_accumulation
    }

    fn teammate_resurrection_accumulation_snapshots(
        &self,
    ) -> &[TeammateResurrectionAccumulationSnapshot] {
        &self.teammate_resurrection_accumulations
    }

    fn begin_map_refresh(&mut self) {
        self.reset_revenant_presence_for_map_refresh();
    }

    fn reset_revenant_presence_for_map_refresh(&mut self) {
        self.revenant_active_summon = None;
        self.revenant_effect_mask = 0;
        self.revenant_summon_handles = [None; REVENANT_SUMMON_COUNT];
        self.revenant_summon_effect_seen = [true; REVENANT_SUMMON_COUNT];
        self.revenant_summon_restore_timer = [0.0; REVENANT_SUMMON_COUNT];
        self.revenant_skill_move_hud = false;
        self.revenant_buddy_count = 0;
        self.revenant_known_buddy_handles.clear();
        self.revenant_bind_baseline.clear();
        self.revenant_pending_summon_bind = None;
        self.revenant_map_refresh_cleanup_pending = true;
    }

    fn finish_revenant_map_refresh_cleanup(&mut self, world_chr_man: &mut HudWorld) {
        if !self.revenant_map_refresh_cleanup_pending {
            return;
        }
        let Some(player) = world_chr_man.player.as_mut() else {
            return;
        };

        remove_revenant_summon_active_effects_from_player(player);
        self.revenant_map_refresh_cleanup_pending = false;
    }

    fn update_revenant_summons(
        &mut self,
        world_chr_man: &mut HudWorld,
        role: Role,
        active_effects: &[i32],
        delta: f32,
    ) {
        if role != Role::Revenant {
            self.revenant_active_summon = None;
            self.revenant_effect_mask = 0;
            self.revenant_summon_handles = [None; REVENANT_SUMMON_COUNT];
            self.revenant_summon_effect_seen = [false; REVENANT_SUMMON_COUNT];
            self.revenant_summon_death_locked = [false; REVENANT_SUMMON_COUNT];
            self.revenant_summon_restore_timer = [0.0; REVENANT_SUMMON_COUNT];
            self.revenant_skill_move_hud = false;
            self.revenant_buddy_count = 0;
            self.revenant_known_buddy_handles.clear();
            self.revenant_bind_baseline.clear();
            self.revenant_pending_summon_bind = None;
            remove_revenant_summon_death_effects(world_chr_man);
            remove_revenant_summon_active_effects(world_chr_man);
            return;
        }

        let current_buddy_handles = revenant_alive_buddy_handles(world_chr_man);
        let buddy_npc_ids = revenant_buddy_param_npc_ids();
        self.revenant_buddy_count = current_buddy_handles.len();
        self.revenant_effect_mask = 0;
        for (index, effect) in REVENANT_SUMMON_EFFECTS.into_iter().enumerate() {
            let active_now = active_effects.contains(&effect);
            if active_now {
                self.revenant_effect_mask |= 1 << index;
            }
            if active_now && !self.revenant_summon_effect_seen[index] {
                self.revenant_active_summon = if let Some(previous) = self.revenant_active_summon {
                    if let Some(hp_ratio) = revenant_buddy_hp_ratio_by_slot(
                        world_chr_man,
                        previous,
                        self.revenant_summon_handles[previous],
                    ) {
                        self.revenant_summon_hp[previous] = hp_ratio;
                    }
                    self.revenant_summon_handles[previous] = None;
                    self.revenant_summon_restore_timer[previous] = 0.0;
                    self.revenant_pending_summon_bind = None;
                    self.revenant_bind_baseline.clear();
                    None
                } else {
                    self.revenant_summon_handles[index] = None;
                    self.revenant_summon_restore_timer[index] = REVENANT_SUMMON_HP_RESTORE_SECONDS;
                    self.revenant_bind_baseline = self.revenant_known_buddy_handles.clone();
                    self.revenant_pending_summon_bind = Some(index);
                    Some(index)
                };
            }
            self.revenant_summon_effect_seen[index] = active_now;
        }

        let mut active_index = self.revenant_active_summon;
        if let Some(index) = active_index {
            let should_restore_hp = self.revenant_pending_summon_bind == Some(index)
                || self.revenant_summon_handles[index].is_none()
                || self.revenant_summon_restore_timer[index] > 0.0;
            let restore_hp_ratio = self.revenant_summon_hp[index].clamp(0.0, 1.0);
            let mut hp_ratio = None;
            let mut removed_by_system = false;

            if let Some(group_chr) =
                revenant_buddy_group_by_param_id(world_chr_man, REVENANT_BUDDY_PARAM_IDS[index])
            {
                if group_chr.retirement_requested && !should_restore_hp {
                    removed_by_system = true;
                } else {
                    if should_restore_hp {
                        apply_revenant_buddy_hp_ratio_address(
                            group_chr.character.address,
                            restore_hp_ratio,
                        );
                    }
                    hp_ratio = if should_restore_hp {
                        Some(restore_hp_ratio)
                    } else {
                        revenant_buddy_hp_ratio_identity(group_chr.character)
                    };
                    if hp_ratio.is_some() {
                        self.revenant_summon_handles[index] = Some(HudHandle::from_native(
                            group_chr.character.native_identifier,
                        ));
                        self.revenant_pending_summon_bind = None;
                        self.revenant_bind_baseline.clear();
                    }
                }
            }

            if !removed_by_system && hp_ratio.is_none() {
                hp_ratio = self.revenant_summon_handles[index].and_then(|handle| {
                    if should_restore_hp {
                        if let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                            apply_revenant_buddy_hp_ratio_address(chr.address, restore_hp_ratio);
                        }
                        Some(restore_hp_ratio)
                    } else {
                        revenant_buddy_hp_ratio_by_handle(world_chr_man, handle)
                    }
                });
            }

            if !removed_by_system && hp_ratio.is_none() {
                let bind_was_pending = self.revenant_pending_summon_bind == Some(index);
                hp_ratio = (|| {
                    let handle = if bind_was_pending {
                        revenant_find_new_buddy_handle(
                            world_chr_man,
                            buddy_npc_ids[index],
                            &current_buddy_handles,
                            &self.revenant_bind_baseline,
                            &self.revenant_summon_handles,
                        )?
                    } else {
                        revenant_find_untracked_buddy_handle(
                            world_chr_man,
                            buddy_npc_ids[index],
                            &current_buddy_handles,
                            &self.revenant_summon_handles,
                        )?
                    };
                    if bind_was_pending
                        && let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                            apply_revenant_buddy_hp_ratio_address(chr.address, restore_hp_ratio);
                        }
                    self.revenant_summon_handles[index] = Some(handle);
                    self.revenant_pending_summon_bind = None;
                    self.revenant_bind_baseline.clear();
                    if bind_was_pending {
                        Some(restore_hp_ratio)
                    } else {
                        revenant_buddy_hp_ratio_by_handle(world_chr_man, handle)
                    }
                })();
            }

            if removed_by_system {
                self.revenant_summon_hp[index] = 0.0;
                self.revenant_summon_handles[index] = None;
                self.revenant_summon_restore_timer[index] = 0.0;
                self.revenant_pending_summon_bind = None;
                self.revenant_bind_baseline.clear();
                self.revenant_active_summon = None;
                active_index = None;
            } else if let Some(hp_ratio) = hp_ratio {
                self.revenant_summon_hp[index] = hp_ratio;
            } else if self.revenant_pending_summon_bind != Some(index)
                && self.revenant_summon_restore_timer[index] <= 0.0
            {
                self.revenant_summon_hp[index] = 0.0;
                self.revenant_summon_handles[index] = None;
                self.revenant_active_summon = None;
                active_index = None;
            } else {
                self.revenant_summon_handles[index] = None;
            }
        }

        for index in 0..REVENANT_SUMMON_COUNT {
            self.revenant_summon_restore_timer[index] =
                (self.revenant_summon_restore_timer[index] - delta).max(0.0);
            if active_index != Some(index) {
                self.revenant_summon_hp[index] = (self.revenant_summon_hp[index]
                    + REVENANT_FAMILY_REGEN_PER_SECOND * delta)
                    .clamp(0.0, 1.0);
            }
        }
        self.sync_revenant_summon_death_effects(world_chr_man);
        let active_effect_index = self.revenant_active_summon.and_then(|index| {
            let hp_ratio = self.revenant_summon_handles[index]
                .and_then(|handle| revenant_buddy_chr_by_handle(world_chr_man, handle))
                .and_then(revenant_buddy_hp_ratio_identity);
            revenant_active_effect_index(Some(index), self.revenant_pending_summon_bind, hp_ratio)
        });
        sync_revenant_summon_active_effects(world_chr_man, active_effect_index);
        if self.revenant_pending_summon_bind.is_none() {
            self.revenant_known_buddy_handles = current_buddy_handles;
        }
    }

    fn sync_revenant_summon_death_effects(&mut self, world_chr_man: &mut HudWorld) {
        let Some(player) = world_chr_man.player.as_mut() else {
            return;
        };
        for (index, &effect) in REVENANT_SUMMON_DEATH_EFFECTS.iter().enumerate() {
            let hp_ratio = self.revenant_summon_hp[index].clamp(0.0, 1.0);
            if hp_ratio <= 0.0 && !self.revenant_summon_death_locked[index] {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::apply_effect(player.actor.identity.address, effect, true)
                    },
                );
                self.revenant_summon_death_locked[index] = true;
            } else if hp_ratio >= REVENANT_SUMMON_REVIVE_READY_HP_RATIO
                && self.revenant_summon_death_locked[index]
            {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(player.actor.identity.address, effect)
                    },
                );
                self.revenant_summon_death_locked[index] = false;
            }
        }
    }

    fn update_revenant_passive(&mut self, world_chr_man: &mut HudWorld, role: Role, now: Instant) {
        if role != Role::Revenant {
            return;
        }

        if !REVENANT_PASSIVE_BUDDY_ROUTE_ENABLED {
            for soul in self.revenant_converted_souls.values() {
                release_revenant_passive_soul(world_chr_man, soul);
            }
            self.revenant_converted_souls.clear();
            self.update_revenant_debug_souls(world_chr_man, now);
            self.submit_pending_revenant_debug_soul(world_chr_man, now);
            self.try_queue_revenant_debug_kill(world_chr_man, now);
            return;
        }

        self.update_revenant_converted_souls(world_chr_man, now);
        self.try_convert_revenant_kills(world_chr_man, now);
    }

    fn update_revenant_converted_souls(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        for soul in self.revenant_converted_souls.values_mut() {
            if soul.handle.is_none() {
                if let Some(group_chr) =
                    revenant_buddy_group_by_param_id(world_chr_man, soul.buddy_param_id)
                {
                    prepare_revenant_passive_buddy(group_chr.character.address);
                    soul.handle = Some(HudHandle::from_native(
                        group_chr.character.native_identifier,
                    ));
                }
            } else if let Some(handle) = soul.handle
                && let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                    prepare_revenant_passive_buddy(chr.address);
                }
        }

        let mut expired = Vec::new();
        for (buddy_param_id, soul) in &self.revenant_converted_souls {
            let should_remove = now >= soul.expires_at
                || soul.handle.is_some_and(|handle| {
                    revenant_buddy_chr_by_handle(world_chr_man, handle)
                        .is_some_and(revenant_converted_soul_identity_should_disappear)
                });
            if should_remove {
                expired.push(*buddy_param_id);
            }
        }

        for buddy_param_id in expired {
            if let Some(soul) = self.revenant_converted_souls.remove(&buddy_param_id) {
                release_revenant_passive_soul(world_chr_man, &soul);
            }
        }
    }

    fn try_convert_revenant_kills(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        let Some(player_handle) = world_chr_man
            .player
            .as_ref()
            .map(|player| player.actor.handle)
        else {
            return;
        };

        let mut candidates = Vec::new();
        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let chr = &copied;
            let handle = chr.handle;
            if self.revenant_seen_deaths.contains(&handle) {
                continue;
            }
            if let Some(candidate) = revenant_passive_candidate(chr, player_handle) {
                candidates.push(candidate);
            }
        }

        for candidate in candidates {
            self.revenant_seen_deaths.insert(candidate.handle);
            if self.revenant_converted_souls.len() >= REVENANT_CONVERTED_SOUL_LIMIT {
                continue;
            }
            if !revenant_passive_roll(candidate.handle, now) {
                continue;
            }

            let Some((buddy_param_id, trigger_effect)) = reserve_revenant_passive_buddy_slot(
                candidate.npc_param_id,
                candidate.npc_think_param_id,
                &self.revenant_converted_souls,
            ) else {
                continue;
            };

            request_revenant_passive_summon(world_chr_man, trigger_effect);
            self.revenant_converted_souls.insert(
                buddy_param_id,
                RevenantConvertedSoul {
                    expires_at: now + Duration::from_secs_f32(REVENANT_CONVERTED_SOUL_SECONDS),
                    buddy_param_id,
                    handle: None,
                },
            );
        }
    }

    fn clear_revenant_converted_souls(&mut self, world_chr_man: &mut HudWorld) {
        for soul in self.revenant_converted_souls.values() {
            release_revenant_passive_soul(world_chr_man, soul);
        }
        self.revenant_converted_souls.clear();
        for soul in self.revenant_debug_souls.drain(..) {
            release_revenant_debug_soul(world_chr_man, &soul);
        }
        self.revenant_pending_debug_soul = None;
        self.revenant_seen_deaths.clear();
    }

    fn update_revenant_debug_souls(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        let world_address = world_chr_man.object.address;
        // SAFETY: Original update callback retains the complete exclusive creation graph/phase.
        if let Err(error) = unsafe { native_creation::poll(world_address, now) } {
            eprintln!(
                "ShikiFSRS HUD creation progress failed or uncertain; ticket retained: {error}"
            );
            return;
        }
        for soul in &mut self.revenant_debug_souls {
            if soul.hide_at.is_some() {
                continue;
            }
            if soul.handle.is_none() {
                match native_creation::result(world_address, soul.creation_request) {
                    Ok(Some(character)) => {
                        prepare_revenant_debug_soul(character.address);
                        request_revenant_debug_soul_animation(
                            character.address,
                            REVENANT_CONVERTED_SOUL_APPEAR_ANIM_ID,
                        );
                        soul.handle = Some(HudHandle::from_native(character.native_identifier));
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        eprintln!("ShikiFSRS HUD attributed creation result rejected: {error}");
                        return;
                    }
                }
            } else if let Some(handle) = soul.handle
                && let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) {
                    prepare_revenant_debug_soul(chr.address);
                }
        }

        let mut index = 0;
        while index < self.revenant_debug_souls.len() {
            let soul = self.revenant_debug_souls[index];
            if let Some(hide_at) = soul.hide_at {
                let died_while_hiding = soul.handle.is_some_and(|handle| {
                    revenant_debug_chr_by_handle(world_chr_man, handle)
                        .is_none_or(revenant_converted_soul_identity_should_disappear)
                });
                if died_while_hiding || now >= hide_at {
                    let soul = self.revenant_debug_souls.swap_remove(index);
                    release_revenant_debug_soul(world_chr_man, &soul);
                } else {
                    index += 1;
                }
                continue;
            }

            let timed_out_unbound =
                soul.handle.is_none() && now.duration_since(soul.requested_at).as_secs_f32() > 2.0;
            let expired_by_time = now >= soul.expires_at || timed_out_unbound;
            let died_or_missing = soul.handle.is_some_and(|handle| {
                revenant_debug_chr_by_handle(world_chr_man, handle)
                    .is_none_or(revenant_converted_soul_identity_should_disappear)
            });
            if died_or_missing {
                let soul = self.revenant_debug_souls.swap_remove(index);
                release_revenant_debug_soul(world_chr_man, &soul);
            } else if expired_by_time {
                if let Some(handle) = soul.handle {
                    if let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) {
                        request_revenant_debug_soul_animation(
                            chr.address,
                            REVENANT_CONVERTED_SOUL_DISAPPEAR_ANIM_ID,
                        );
                    }
                    self.revenant_debug_souls[index].hide_at = Some(
                        now + Duration::from_secs_f32(REVENANT_CONVERTED_SOUL_END_ANIM_SECONDS),
                    );
                    index += 1;
                } else {
                    let soul = self.revenant_debug_souls.swap_remove(index);
                    release_revenant_debug_soul(world_chr_man, &soul);
                }
            } else {
                index += 1;
            }
        }
    }

    fn submit_pending_revenant_debug_soul(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        let Some(pending) = self.revenant_pending_debug_soul else {
            return;
        };
        let has_unbound_request = self
            .revenant_debug_souls
            .iter()
            .any(|soul| soul.handle.is_none());
        let world_address = world_chr_man.object.address;
        let creator_busy = match native_creation::busy(world_address) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("ShikiFSRS HUD creator observation rejected: {error}");
                return;
            }
        };
        if !revenant_debug_spawn_submission_ready(
            creator_busy,
            has_unbound_request,
            now,
            pending.ready_at,
        ) {
            return;
        }

        if revenant_passive_candidate_validation(&pending.candidate).is_err() {
            self.revenant_pending_debug_soul = None;
            return;
        }

        let candidate = pending.candidate;
        // SAFETY: Preserve original ordinary enemy request in the exclusive creation callback.
        // Enqueue returns the owned ID before progress can attempt any native operation.
        let creation_request = match unsafe {
            native_creation::submit(
                world_address,
                shikifsrs_elden_ring::creation_input_preparation::OrdinaryCreationInput {
                    name_units: format!("c{:04}", candidate.npc_id).encode_utf16().collect(),
                    position_bits: [
                        candidate.position.0,
                        candidate.position.1,
                        candidate.position.2,
                        candidate.position.3,
                    ]
                    .map(f32::to_bits),
                    rotation_bits: [0; 4],
                    chara_init_param_id: 0,
                    npc_param_id: candidate.npc_param_id,
                    npc_think_param_id: candidate.npc_think_param_id,
                    event_entity_id: 0,
                    talk_id: 0,
                    animation_id: -1,
                    route:
                        shikifsrs_elden_ring::debug_creation_fields::OrdinaryCreationRoute::Enemy,
                },
                now,
            )
        } {
            Ok(request) => request,
            Err(error) => {
                eprintln!("ShikiFSRS HUD creation enqueue rejected: {error}");
                return;
            }
        };
        self.revenant_debug_souls.push(RevenantDebugSoul {
            expires_at: now + Duration::from_secs_f32(REVENANT_CONVERTED_SOUL_SECONDS),
            requested_at: now,
            handle: None,
            creation_request,
            npc_id: candidate.npc_id,
            npc_param_id: candidate.npc_param_id,
            hide_at: None,
        });
        self.revenant_pending_debug_soul = None;
    }

    fn try_queue_revenant_debug_kill(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        let Some(player_handle) = world_chr_man
            .player
            .as_ref()
            .map(|player| player.actor.handle)
        else {
            return;
        };

        if self.revenant_pending_debug_soul.is_some()
            || self.revenant_debug_souls.len() >= REVENANT_CONVERTED_SOUL_LIMIT
        {
            return;
        }

        let mut candidates = Vec::new();
        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let chr = &copied;
            let handle = chr.handle;
            if self.revenant_seen_deaths.contains(&handle) {
                continue;
            }
            if let Some(candidate) = revenant_passive_candidate(chr, player_handle) {
                candidates.push(candidate);
            }
        }

        for candidate in candidates {
            self.revenant_seen_deaths.insert(candidate.handle);
            if self.revenant_debug_souls.len() >= REVENANT_CONVERTED_SOUL_LIMIT {
                break;
            }
            if !revenant_passive_roll(candidate.handle, now) {
                continue;
            }

            self.revenant_pending_debug_soul = Some(RevenantPendingDebugSoul {
                candidate,
                ready_at: now + REVENANT_DEBUG_SPAWN_DELAY,
            });
            break;
        }
    }

    fn begin_undertaker_skill(&mut self, enhanced: bool, now: Instant) -> i32 {
        let (duration, effect) = if enhanced {
            (
                UNDERTAKER_ENHANCED_BUFF_SECONDS,
                SP_UNDERTAKER_ENHANCED_SKILL_BUFF,
            )
        } else {
            (UNDERTAKER_BUFF_SECONDS, SP_UNDERTAKER_NORMAL_SKILL_BUFF)
        };
        self.undertaker_buff_duration = duration;
        self.undertaker_buff_end = Some(now + Duration::from_secs_f32(duration));
        self.skill_layers = [1.0, 0.0];
        effect
    }

    fn advance_skill(&mut self, config: &RoleConfig, now: Instant, delta: f32) {
        match config.skill_kind {
            SkillKind::NoCooldown => {
                self.skill_layers = [1.0, 0.0];
            }
            SkillKind::Timed => {
                advance_layered_charge(
                    &mut self.skill_layers,
                    config.skill_charges,
                    delta / config.skill_cooldown,
                );
            }
            SkillKind::UndertakerBuff => {
                if let Some(end) = self.undertaker_buff_end {
                    let remaining = (end - now).as_secs_f32();
                    if remaining <= 0.0 {
                        self.undertaker_buff_end = None;
                        self.skill_layers = [0.0, 0.0];
                        self.undertaker_auto_end_pending = true;
                    } else {
                        self.skill_layers = [
                            (remaining / self.undertaker_buff_duration).clamp(0.0, 1.0),
                            0.0,
                        ];
                    }
                } else {
                    advance_layered_charge(
                        &mut self.skill_layers,
                        1,
                        delta / config.skill_cooldown,
                    );
                }
            }
        }
    }

    fn advance_ultimate(&mut self, config: &RoleConfig, now: Instant, delta: f32) {
        if config.role == Role::Undertaker
            && self
                .undertaker_free_ultimate_end
                .is_some_and(|end| (end - now).as_secs_f32() <= 0.0)
            {
                self.undertaker_free_ultimate_end = None;
            }

        if config.role == Role::Executor
            && let Some(end) = self.executor_transform_end {
                let remaining = (end - now).as_secs_f32();
                if remaining <= 0.0 {
                    self.executor_transform_end = None;
                    self.ultimate = 0.0;
                    self.executor_auto_end_pending = true;
                } else {
                    self.ultimate = (remaining / EXECUTOR_TRANSFORM_SECONDS).clamp(0.0, 1.0);
                }
                return;
            }

        self.ultimate = (self.ultimate + delta / config.ultimate_cooldown).clamp(0.0, 1.0);
    }

    fn apply_pending_undertaker_events(&mut self, player: &mut HudPlayer, config: &RoleConfig) {
        if config.role == Role::Undertaker && self.undertaker_auto_end_pending {
            clear_undertaker_skill_effects(player);
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        player.actor.identity.address,
                        config.effect(SP_UNDERTAKER_SKILL_AUTO_END),
                        true,
                    )
                },
            );
            self.undertaker_auto_end_pending = false;
        }
    }

    fn apply_pending_executor_events(&mut self, player: &mut HudPlayer, config: &RoleConfig) {
        if config.role == Role::Executor && self.executor_auto_end_pending {
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        player.actor.identity.address,
                        config.effect(SP_EXECUTOR_ULTIMATE_AUTO_END),
                        true,
                    )
                },
            );
            self.executor_auto_end_pending = false;
        }
    }

    fn log_damage_diagnostics(&mut self, now: Instant) {
        if (now - self.last_damage_log).as_secs_f32() < DAMAGE_LOG_INTERVAL {
            return;
        }
        self.last_damage_log = now;

        let calls = DAMAGE_HOOK_CALLS.swap(0, Ordering::Relaxed);
        let positive = DAMAGE_HOOK_POSITIVE.swap(0, Ordering::Relaxed);
        let excluded = DAMAGE_HOOK_EXCLUDED_LOCAL.swap(0, Ordering::Relaxed);
        if calls == 0 && positive == 0 && excluded == 0 {
            return;
        }

        append_damage_log(&format!(
            "[damage] calls={calls} positive={positive} excluded_local={excluded} local=0x{:X} last_target=0x{:X} last_r8=0x{:X} last_offset=0x{:X} last_value={}",
            LOCAL_DAMAGE_MODULE.load(Ordering::Relaxed),
            DAMAGE_HOOK_LAST_TARGET.load(Ordering::Relaxed),
            DAMAGE_HOOK_LAST_DATA.load(Ordering::Relaxed),
            DAMAGE_HOOK_LAST_OFFSET.load(Ordering::Relaxed),
            DAMAGE_HOOK_LAST_VALUE.load(Ordering::Relaxed),
        ));
    }

    fn consume_damage_events(&mut self, now: Instant) {
        let hits = ULTIMATE_DAMAGE_EVENTS.swap(0, Ordering::Relaxed);
        if self.executor_transform_end.is_some() {
            return;
        }
        if hits > 0 && self.hit_gain_ready(now) {
            self.ultimate = (self.ultimate + ULTIMATE_HIT_GAIN).clamp(0.0, 1.0);
            self.last_hit_gain = now;
        }
    }

    fn update_scholar_targets(
        &mut self,
        world_chr_man: &HudWorld,
        role: Role,
        active_effects: &[i32],
        delta: f32,
    ) {
        self.scholar_observing = role == Role::Scholar
            && active_effects.contains(&role_config(Role::Scholar).effect(SP_SCHOLAR_OBSERVE));
        for target in self.scholar_targets.values_mut() {
            target.visible = false;
        }
        self.scholar_debug = ScholarScanDebug::default();

        if self.scholar_observing {
            let Some(player) = world_chr_man.player else {
                self.decay_scholar_targets(world_chr_man, delta);
                return;
            };
            let player_pos = hud_physics_position(&player.actor);

            let entries = match native_world::update_entries(world_chr_man.object.address) {
                Ok(entries) => entries,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                    return;
                }
            };
            for entry in entries.into_iter().take(SCHOLAR_SCAN_MAX_DISTANCE_ENTRIES) {
                self.scholar_debug.scanned += 1;

                if let Err(error) = native_world::revalidate(entry.character) {
                    eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                    return;
                }
                // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
                // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
                let copied = match HudActor::observe(entry.character) {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                        return;
                    }
                };
                let chr = &copied;
                if let Some(reason) = scholar_reject_reason(chr) {
                    match reason {
                        ScholarRejectReason::Type => self.scholar_debug.type_skip += 1,
                        ScholarRejectReason::Hidden => self.scholar_debug.hidden_skip += 1,
                        ScholarRejectReason::Hp => self.scholar_debug.hp_skip += 1,
                    }
                    continue;
                }
                let Some(screen_pos) = scholar_target_screen_pos(chr) else {
                    self.scholar_debug.screen_skip += 1;
                    continue;
                };
                if !scholar_screen_pos_in_lens(screen_pos) {
                    self.scholar_debug.screen_skip += 1;
                    continue;
                }

                let target_pos = scholar_chr_physics_position(chr);
                let target_distance = distance_havok(player_pos, target_pos);
                if !scholar_in_lock_target_volume(&player, target_pos, target_distance) {
                    self.scholar_debug.range_skip += 1;
                    continue;
                }
                let scan_gain = scholar_scan_gain(target_distance);
                if scan_gain <= 0.0 {
                    self.scholar_debug.range_skip += 1;
                    continue;
                }
                if !scholar_has_line_of_sight(chr, &player) {
                    self.scholar_debug.los_skip += 1;
                    continue;
                }
                let gain = scan_gain * delta;
                let target = self.scholar_targets.entry(chr.handle).or_default();
                target.progress = (target.progress + gain).clamp(0.0, 1.0);
                target.pos = screen_pos;
                target.visible = true;
                target.handle = Some(chr.handle);
                self.scholar_debug.accepted += 1;
            }
        }

        self.decay_scholar_targets(world_chr_man, delta);
    }

    fn decay_scholar_targets(&mut self, world_chr_man: &HudWorld, delta: f32) {
        let mut expired = Vec::new();
        for (handle, target) in self.scholar_targets.iter_mut() {
            if !target.visible {
                let Some(chr) = hud_actor_by_handle(world_chr_man, handle) else {
                    expired.push(*handle);
                    continue;
                };
                if scholar_reject_reason(&chr).is_some() {
                    expired.push(*handle);
                    continue;
                }
                let Some(screen_pos) = scholar_target_screen_pos(&chr) else {
                    expired.push(*handle);
                    continue;
                };
                if !scholar_screen_pos_on_view(screen_pos) {
                    expired.push(*handle);
                    continue;
                }
                target.pos = screen_pos;
                target.progress = (target.progress - SCHOLAR_SCAN_DECAY * delta).clamp(0.0, 1.0);
            }
        }
        for handle in expired {
            self.scholar_targets.remove(&handle);
        }
        self.scholar_targets
            .retain(|_, target| target.progress > 0.001 || target.visible);
    }

    fn update_wylder_skill_lock(
        &mut self,
        world_chr_man: &HudWorld,
        role: Role,
        active_effects: &[i32],
    ) {
        self.wylder_skill_lock_active = role == Role::Wylder
            && active_effects.contains(&role_config(Role::Wylder).effect(SP_WYLDER_SKILL_LOCK));
        let target = self
            .wylder_skill_lock_active
            .then(|| wylder_skill_lock_target(world_chr_man))
            .flatten();
        self.wylder_skill_lock_pos = target.map(|target| target.screen_pos);
        self.wylder_skill_lock_scale = target
            .map(|target| wylder_skill_lock_distance_scale(target.distance))
            .unwrap_or(1.0);
        self.wylder_skill_lock_has_target = self.wylder_skill_lock_pos.is_some();
        if self.wylder_skill_lock_active {
            self.wylder_hookshot_aim_target = target;
        }
    }

    fn update_wylder_hookshot_native_resolution(
        &mut self,
        world_chr_man: &mut HudWorld,
        active_effects: &[i32],
    ) {
        let active = active_effects.contains(&SP_WYLDER_HOOKSHOT_RESOLVE_ACTIVE);
        let started = active && !self.wylder_hookshot_native.active;
        if started {
            self.wylder_hookshot_target = None;
            clear_wylder_hookshot_player_outputs(world_chr_man);
        }
        if !active {
            self.wylder_hookshot_native.update(false, None);
            clear_wylder_hookshot_player_outputs(world_chr_man);
            return;
        }
        if self.wylder_hookshot_native.is_resolved() {
            return;
        }

        let enemy = active
            .then(|| wylder_hookshot_target_result(world_chr_man))
            .flatten();
        let emitted = self
            .wylder_hookshot_native
            .update(active, enemy.map(|(_, result)| result));

        if let Some((handle, _)) = enemy {
            self.wylder_hookshot_target = Some(handle);
            if let Some(target) = hud_actor_by_handle(world_chr_man, &handle) {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            target.identity.address,
                            SP_WYLDER_HOOKSHOT_IMPACT_ACCEPTED,
                        )
                    },
                );
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            target.identity.address,
                            SP_WYLDER_HOOKSHOT_IMPACT_REJECTED,
                        )
                    },
                );
            }
        }

        let Some(result) = emitted else {
            return;
        };
        clear_wylder_hookshot_player_outputs(world_chr_man);
        let Some(player) = world_chr_man.player.as_mut() else {
            return;
        };
        let output = match result {
            WylderHookshotNativeResult::PullEnemy => SP_WYLDER_HOOKSHOT_PULL_ENEMY,
            WylderHookshotNativeResult::PullToEnemy => SP_WYLDER_HOOKSHOT_PULL_TO_ENEMY,
        };
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::apply_effect(player.actor.identity.address, output, true) },
        );
    }

    fn update_wylder_hookshot_movement_stop(
        &mut self,
        world_chr_man: &mut HudWorld,
        active_effects: &[i32],
        wylder_active: bool,
        now: Instant,
    ) {
        let active = wylder_active && active_effects.contains(&SP_WYLDER_HOOKSHOT_MOVEMENT_ACTIVE);
        if !active {
            let was_active = self.wylder_hookshot_movement_stop.active;
            self.wylder_hookshot_movement_stop.reset();
            if let Some(player) = world_chr_man.player.as_mut() {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            player.actor.identity.address,
                            SP_WYLDER_HOOKSHOT_MOVEMENT_STOP,
                        )
                    },
                );
            }
            if was_active {
                self.wylder_hookshot_target = None;
                self.wylder_hookshot_aim_target = None;
            }
            return;
        }

        let Some((player_position, player_capsule)) = world_chr_man.player.map(|player| {
            (
                scholar_chr_physics_position(&player.actor),
                wylder_hookshot_chr_capsule(&player.actor),
            )
        }) else {
            self.wylder_hookshot_movement_stop.reset();
            return;
        };
        let candidate = self.wylder_hookshot_movement_destination(active_effects);
        if !self.wylder_hookshot_movement_stop.active {
            self.wylder_hookshot_movement_stop
                .begin(candidate, player_position, now);
            if let Some(player) = world_chr_man.player.as_mut() {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            player.actor.identity.address,
                            SP_WYLDER_HOOKSHOT_MOVEMENT_STOP,
                        )
                    },
                );
            }
        } else {
            self.wylder_hookshot_movement_stop
                .set_destination_if_missing(candidate, player_position, now);
        }

        let target = self
            .wylder_hookshot_movement_stop
            .destination
            .and_then(|destination| wylder_hookshot_destination_sample(world_chr_man, destination));
        let target_position = target.map(|(position, _)| position);
        let target_capsule = target.and_then(|(_, capsule)| capsule);
        if !self.wylder_hookshot_movement_stop.sample(
            player_position,
            target_position,
            player_capsule,
            target_capsule,
            now,
        ) {
            return;
        }
        if let Some(player) = world_chr_man.player.as_mut() {
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        player.actor.identity.address,
                        SP_WYLDER_HOOKSHOT_MOVEMENT_STOP,
                        true,
                    )
                },
            );
        }
    }

    fn wylder_hookshot_movement_destination(
        &self,
        active_effects: &[i32],
    ) -> Option<WylderHookshotMovementDestination> {
        let aimed_enemy = self.wylder_hookshot_aim_target.and_then(|target| {
            if let WylderSkillLockTargetKind::Enemy(handle) = target.kind {
                Some(WylderHookshotMovementDestination::Enemy(handle))
            } else {
                None
            }
        });
        let aimed_map = self.wylder_hookshot_aim_target.and_then(|target| {
            (target.kind == WylderSkillLockTargetKind::Map)
                .then_some(WylderHookshotMovementDestination::Point(target.world_pos))
        });
        let resolved_enemy = self
            .wylder_hookshot_target
            .map(WylderHookshotMovementDestination::Enemy);

        if active_effects.contains(&SP_WYLDER_HOOKSHOT_PULL_TO_ENEMY) {
            return resolved_enemy.or(aimed_enemy);
        }
        if active_effects.contains(&SP_WYLDER_HOOKSHOT_PULL_TO_MAP) {
            return aimed_map;
        }
        resolved_enemy.or(aimed_enemy)
    }

    fn update_ironeye_weakness(
        &mut self,
        world_chr_man: &mut HudWorld,
        role: Role,
        now: Instant,
        damage_events: &[DamageHookEvent],
    ) {
        let config = role_config(Role::Ironeye);
        if role != Role::Ironeye {
            self.clear_ironeye_weakness_marks(world_chr_man, config);
            self.ironeye_weakness_bursts.clear();
            return;
        }

        self.ironeye_weakness_bursts.retain(|burst| {
            (now - burst.created_at).as_secs_f32() < IRONEYE_WEAKNESS_BURST_SECONDS
        });
        self.expire_ironeye_weakness_marks(world_chr_man, config, now);
        self.refresh_ironeye_weakness_marks(world_chr_man, config, now);
        self.accumulate_ironeye_weakness_damage(world_chr_man, config, now, damage_events);
    }

    fn refresh_ironeye_weakness_marks(
        &mut self,
        world_chr_man: &mut HudWorld,
        config: &RoleConfig,
        now: Instant,
    ) {
        let trigger = config.effect(SP_IRONEYE_WEAKNESS_TRIGGER);
        let mut handles = Vec::new();
        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let chr = &copied;
            if scholar_reject_reason(chr).is_some() || !chr_has_speffect(chr, trigger) {
                continue;
            }
            handles.push(chr.handle);
        }

        for handle in handles {
            let Some(enemy) = hud_actor_by_handle(world_chr_man, &handle) else {
                continue;
            };
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe { native_character::remove_effect(enemy.identity.address, trigger) },
            );
            if scholar_reject_reason(&enemy).is_some() {
                continue;
            }
            let data = match native_vitals::observe(enemy.identity.address) {
                Ok(value) => value.hp,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD weakness vital copy rejected: {error}");
                    continue;
                }
            };
            let damage_module = damage_module_from_chr_ins(&enemy);
            if damage_module == 0 || data.maximum <= 0 {
                continue;
            }
            let threshold_damage =
                ((data.maximum as f32) * IRONEYE_WEAKNESS_THRESHOLD_MAX_HP_RATE).ceil() as i32;
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        enemy.identity.address,
                        config.effect(SP_IRONEYE_WEAKNESS_ACTIVE),
                        true,
                    )
                },
            );
            self.ironeye_weakness_marks
                .entry(handle)
                .and_modify(|mark| {
                    mark.damage_module = damage_module;
                    mark.threshold_damage = threshold_damage.max(1);
                    mark.expires_at = now + Duration::from_secs_f32(IRONEYE_WEAKNESS_DURATION);
                })
                .or_insert(IroneyeWeaknessMark {
                    damage_module,
                    accumulated_damage: 0,
                    threshold_damage: threshold_damage.max(1),
                    started_at: now,
                    expires_at: now + Duration::from_secs_f32(IRONEYE_WEAKNESS_DURATION),
                });
        }
    }

    fn accumulate_ironeye_weakness_damage(
        &mut self,
        world_chr_man: &mut HudWorld,
        config: &RoleConfig,
        now: Instant,
        events: &[DamageHookEvent],
    ) {
        if events.is_empty() || self.ironeye_weakness_marks.is_empty() {
            return;
        }

        let mut broken = Vec::new();
        for event in events.iter().copied() {
            for (handle, mark) in self.ironeye_weakness_marks.iter_mut() {
                if event.target_damage_module != mark.damage_module
                    || event.created_at < mark.started_at
                    || now >= mark.expires_at
                {
                    continue;
                }
                mark.accumulated_damage =
                    mark.accumulated_damage.saturating_add(event.damage.max(0));
                if mark.accumulated_damage >= mark.threshold_damage {
                    broken.push(*handle);
                }
            }
        }

        broken.sort_by_key(|handle| (handle.diagnostic_set_index(), handle.diagnostic_ordinal()));
        broken.dedup();
        for handle in broken {
            self.break_ironeye_weakness_mark(world_chr_man, config, handle);
        }
    }

    fn break_ironeye_weakness_mark(
        &mut self,
        world_chr_man: &mut HudWorld,
        config: &RoleConfig,
        handle: HudHandle,
    ) {
        if self.ironeye_weakness_marks.remove(&handle).is_none() {
            return;
        }
        let burst_pos = world_chr_man.player.and_then(|player| {
            hud_actor_by_handle(world_chr_man, &handle)
                .and_then(|target| ironeye_weakness_marker_screen_pos(&player, &target))
        });
        if let Some(enemy) = hud_actor_by_handle(world_chr_man, &handle) {
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::remove_effect(
                        enemy.identity.address,
                        config.effect(SP_IRONEYE_WEAKNESS_ACTIVE),
                    )
                },
            );
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        enemy.identity.address,
                        config.effect(SP_IRONEYE_WEAKNESS_BREAK),
                        true,
                    )
                },
            );
        }
        if let Some(pos) = burst_pos {
            if self.ironeye_weakness_bursts.len() >= IRONEYE_WEAKNESS_MAX_BURSTS {
                self.ironeye_weakness_bursts.remove(0);
            }
            self.ironeye_weakness_bursts.push(IroneyeWeaknessBurst {
                pos,
                created_at: Instant::now(),
            });
        }
    }

    fn expire_ironeye_weakness_marks(
        &mut self,
        world_chr_man: &mut HudWorld,
        config: &RoleConfig,
        now: Instant,
    ) {
        let mut expired = Vec::new();
        for (&handle, mark) in self.ironeye_weakness_marks.iter() {
            let invalid = hud_actor_by_handle(world_chr_man, &handle)
                .is_none_or(|enemy| scholar_reject_reason(&enemy).is_some());
            if now >= mark.expires_at || invalid {
                expired.push(handle);
            }
        }
        for handle in expired {
            self.ironeye_weakness_marks.remove(&handle);
            if let Some(enemy) = hud_actor_by_handle(world_chr_man, &handle) {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            enemy.identity.address,
                            config.effect(SP_IRONEYE_WEAKNESS_ACTIVE),
                        )
                    },
                );
            }
        }
    }

    fn clear_ironeye_weakness_marks(&mut self, world_chr_man: &mut HudWorld, config: &RoleConfig) {
        for handle in self
            .ironeye_weakness_marks
            .drain()
            .map(|(handle, _)| handle)
        {
            if let Some(enemy) = hud_actor_by_handle(world_chr_man, &handle) {
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::remove_effect(
                            enemy.identity.address,
                            config.effect(SP_IRONEYE_WEAKNESS_ACTIVE),
                        )
                    },
                );
            }
        }
    }

    fn update_recluse_blood_song(
        &mut self,
        world_chr_man: &mut HudWorld,
        now: Instant,
        damage_events: &[DamageHookEvent],
    ) {
        self.refresh_recluse_blood_brands(world_chr_man);
        self.recluse_blood_debug.branded = self.recluse_blood_brands.len();
        self.recluse_blood_debug.hook_events = damage_events.len();
        self.queue_recluse_blood_damage_events(damage_events);
        self.apply_recluse_blood_song_rewards(world_chr_man, now);
    }

    fn queue_recluse_blood_damage_events(&mut self, damage_events: &[DamageHookEvent]) {
        self.recluse_blood_debug.queued = 0;
        if damage_events.is_empty() || self.recluse_blood_brands.is_empty() {
            self.recluse_blood_debug.pending = self.recluse_blood_pending_damage.len();
            return;
        }
        for event in damage_events.iter().copied() {
            if event.damage <= 0 {
                continue;
            }
            let Some(brand) = self
                .recluse_blood_brands
                .values()
                .find(|brand| brand.damage_module == event.target_damage_module)
            else {
                continue;
            };
            self.recluse_blood_pending_damage
                .push(RecluseBloodDamageEvent {
                    target_damage_module: event.target_damage_module,
                    damage: event.damage,
                    created_at: event.created_at,
                    brand_kind: brand.kind,
                });
            self.recluse_blood_debug.queued += 1;
        }
        self.recluse_blood_debug.pending = self.recluse_blood_pending_damage.len();
    }

    fn apply_recluse_blood_song_rewards(&mut self, world_chr_man: &mut HudWorld, now: Instant) {
        self.recluse_blood_debug.resolved = 0;
        self.recluse_blood_debug.expired = 0;
        if self.recluse_blood_pending_damage.is_empty() {
            return;
        }
        let Some((player_handle, player_team_type)) = world_chr_man
            .player
            .as_ref()
            .map(|player| (player.actor.handle, player.actor.team))
        else {
            return;
        };

        let mut restores: HashMap<HudHandle, (i32, i32)> = HashMap::new();
        let pending_events = std::mem::take(&mut self.recluse_blood_pending_damage);
        let mut retained = Vec::new();
        for event in pending_events {
            let age = (now - event.created_at).as_secs_f32();
            if age < RECLUSE_BLOOD_SOUL_DAMAGE_RESOLVE_DELAY {
                retained.push(event);
                continue;
            }
            if age > RECLUSE_BLOOD_SOUL_DAMAGE_EVENT_TTL {
                self.recluse_blood_debug.expired += 1;
                continue;
            }
            let Some(handle) = self
                .recluse_blood_brands
                .iter()
                .find(|(_, brand)| brand.damage_module == event.target_damage_module)
                .map(|(handle, _)| *handle)
            else {
                continue;
            };
            let Some(enemy) = hud_actor_by_handle(world_chr_man, &handle) else {
                self.recluse_blood_debug.expired += 1;
                continue;
            };
            let raw_attacker = enemy.last_effect_source;
            let attacker = if raw_attacker.is_empty() {
                player_handle
            } else {
                raw_attacker
            };
            let attacker_friendly = recluse_blood_attacker_is_friendly(
                world_chr_man,
                attacker,
                player_handle,
                player_team_type,
            );
            self.recluse_blood_debug.last_attacker_container = attacker.diagnostic_set_index();
            self.recluse_blood_debug.last_attacker_index = attacker.diagnostic_ordinal();
            self.recluse_blood_debug.last_attacker_friendly = attacker_friendly;
            if !attacker_friendly {
                retained.push(event);
                continue;
            }
            let damage = event.damage.max(0);
            if damage <= 0 {
                continue;
            }
            let (hp_restore, fp_restore) = match event.brand_kind {
                RecluseBloodBrandKind::Normal => (
                    scale_i32(damage, RECLUSE_BLOOD_SOUL_HP_RESTORE_RATE),
                    scale_i32(damage, RECLUSE_BLOOD_SOUL_FP_RESTORE_RATE),
                ),
                RecluseBloodBrandKind::FixedFpOnly => {
                    if attacker != player_handle {
                        continue;
                    }
                    (
                        0,
                        fixed_fp_restore_for_player(
                            world_chr_man,
                            &mut self.recluse_blood_fixed_fp_accumulator,
                        ),
                    )
                }
            };
            self.recluse_blood_debug.last_damage = damage;
            self.recluse_blood_debug.last_hp_restore = hp_restore;
            self.recluse_blood_debug.last_fp_restore = fp_restore;
            self.recluse_blood_debug.resolved += 1;
            let entry = restores.entry(attacker).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(hp_restore);
            entry.1 = entry.1.saturating_add(fp_restore);
        }
        self.recluse_blood_pending_damage = retained;
        self.recluse_blood_debug.pending = self.recluse_blood_pending_damage.len();

        for (attacker, (hp_restore, fp_restore)) in restores {
            if hp_restore > 0 {
                apply_hp_delta_by_handle(world_chr_man, attacker, hp_restore);
            }
            if fp_restore > 0 {
                apply_fp_delta_by_handle(world_chr_man, attacker, fp_restore);
            }
        }
    }

    fn refresh_recluse_blood_brands(&mut self, world_chr_man: &HudWorld) {
        let brand_sp_effect = role_config(Role::Recluse).effect(SP_RECLUSE_BLOOD_SOUL_BRAND);
        let fixed_fp_sp_effect = SP_RECLUSE_BLOOD_SOUL_FIXED_FP_BRAND;
        let mut seen = Vec::new();
        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let enemy = &copied;
            if scholar_reject_reason(enemy).is_some() {
                continue;
            }
            let has_normal_brand = chr_has_speffect(enemy, brand_sp_effect);
            let has_fixed_fp_brand = chr_has_speffect(enemy, fixed_fp_sp_effect);
            if !has_normal_brand && !has_fixed_fp_brand {
                continue;
            }
            let damage_module = damage_module_from_chr_ins(enemy);
            if damage_module == 0 {
                continue;
            }
            let handle = enemy.handle;
            let kind = if has_normal_brand {
                RecluseBloodBrandKind::Normal
            } else {
                RecluseBloodBrandKind::FixedFpOnly
            };
            self.recluse_blood_brands.insert(
                handle,
                RecluseBloodBrand {
                    damage_module,
                    kind,
                },
            );
            seen.push(handle);
        }
        self.recluse_blood_brands
            .retain(|handle, _| seen.contains(handle));
    }

    fn update_duchess_replay_history(&mut self, world_chr_man: &HudWorld, role: Role) {
        if role != Role::Duchess {
            self.duchess_snapshots.clear();
            return;
        }

        let mut seen = Vec::new();
        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let enemy = &copied;
            if scholar_reject_reason(enemy).is_some() {
                continue;
            }
            let damage_module = damage_module_from_chr_ins(enemy);
            if damage_module == 0 {
                continue;
            }
            let snapshot = duchess_replay_snapshot(enemy);
            if let Some(previous) = self.duchess_snapshots.get(&damage_module).copied() {
                let payload = snapshot.payload_since(previous);
                if payload.has_effect() {
                    record_duchess_replay_event(damage_module, payload);
                }
            }
            self.duchess_snapshots.insert(damage_module, snapshot);
            seen.push(damage_module);
        }
        self.duchess_snapshots
            .retain(|damage_module, _| seen.contains(damage_module));
    }

    fn update_scholar_sympathy(
        &mut self,
        world_chr_man: &mut HudWorld,
        role: Role,
        active_effects: &[i32],
        locked_on_enemy: HudHandle,
        now: Instant,
    ) {
        self.prune_scholar_damage_numbers(now);
        self.prune_scholar_native_damage_ignores(now);
        if role != Role::Scholar {
            self.scholar_sympathy_hp.clear();
            self.scholar_damage_numbers.clear();
            self.scholar_native_damage_ignores.clear();
            return;
        }

        let config = role_config(Role::Scholar);
        let links = collect_scholar_links(world_chr_man, config, active_effects);
        if links.is_empty() {
            self.scholar_sympathy_hp.clear();
            return;
        }

        self.apply_scholar_sympathy_hp_losses(world_chr_man, &links, locked_on_enemy, now);
        self.apply_scholar_sympathy_heals(world_chr_man, &links, now);
        self.refresh_scholar_sympathy_hp(world_chr_man, &links);
    }

    fn prune_scholar_damage_numbers(&mut self, now: Instant) {
        self.scholar_damage_numbers.retain(|number| {
            (now - number.created_at).as_secs_f32() < SCHOLAR_DAMAGE_NUMBER_SECONDS
        });
    }

    fn prune_scholar_native_damage_ignores(&mut self, now: Instant) {
        self.scholar_native_damage_ignores
            .retain(|_, (_, expires_at)| *expires_at > now);
    }

    fn apply_scholar_sympathy_hp_losses(
        &mut self,
        world_chr_man: &mut HudWorld,
        links: &[ScholarLink],
        locked_on_enemy: HudHandle,
        now: Instant,
    ) {
        for source in links {
            let Some(previous_hp) = self.scholar_sympathy_hp.get(&source.handle).copied() else {
                continue;
            };
            let mut damage = previous_hp - source.hp;
            if damage <= 0 {
                continue;
            }
            damage -= self.consume_scholar_native_damage_ignore(source.handle, damage);
            if damage <= 0 {
                continue;
            }

            match source.kind {
                ScholarLinkKind::Enemy => {
                    self.apply_scholar_sympathy_enemy_damage(
                        world_chr_man,
                        links,
                        source,
                        damage,
                        now,
                    );
                }
                ScholarLinkKind::SelfLink => {
                    self.apply_scholar_sympathy_self_counter(
                        world_chr_man,
                        links,
                        source,
                        damage,
                        locked_on_enemy,
                        now,
                    );
                }
                ScholarLinkKind::Ally => {
                    self.apply_scholar_sympathy_ally_counter(
                        world_chr_man,
                        links,
                        source,
                        damage,
                        now,
                    );
                }
            }
        }
    }

    fn apply_scholar_sympathy_enemy_damage(
        &mut self,
        world_chr_man: &mut HudWorld,
        links: &[ScholarLink],
        source: &ScholarLink,
        damage: i32,
        now: Instant,
    ) {
        let spread_damage = scholar_scaled_amount(damage, SCHOLAR_SYMPATHY_DAMAGE_SPREAD_RATE);
        if spread_damage > 0 {
            for enemy in links
                .iter()
                .filter(|link| link.kind == ScholarLinkKind::Enemy && link.handle != source.handle)
            {
                self.apply_scholar_popup_damage(world_chr_man, enemy.handle, spread_damage, now);
            }
        }

        let ally_heal = scholar_scaled_amount(damage, SCHOLAR_SYMPATHY_ATTACK_HEAL_RATE);
        if ally_heal > 0 {
            for ally in links.iter().filter(|link| scholar_link_is_friendly(link)) {
                apply_hp_delta_by_handle(world_chr_man, ally.handle, ally_heal);
            }
        }
    }

    fn apply_scholar_sympathy_self_counter(
        &mut self,
        world_chr_man: &mut HudWorld,
        links: &[ScholarLink],
        source: &ScholarLink,
        damage: i32,
        locked_on_enemy: HudHandle,
        now: Instant,
    ) {
        let Some(attacker) = scholar_counter_attacker(
            world_chr_man,
            links,
            source.handle,
            source.last_hit_by,
            (!locked_on_enemy.is_empty()).then_some(locked_on_enemy),
        ) else {
            return;
        };
        let counter_damage = scholar_scaled_amount(damage, SCHOLAR_SYMPATHY_COUNTER_RATE);
        if counter_damage <= 0 {
            return;
        }
        self.apply_scholar_popup_damage(world_chr_man, attacker, counter_damage, now);
    }

    fn apply_scholar_sympathy_ally_counter(
        &mut self,
        world_chr_man: &mut HudWorld,
        links: &[ScholarLink],
        source: &ScholarLink,
        damage: i32,
        now: Instant,
    ) {
        let Some(attacker) = scholar_counter_attacker(
            world_chr_man,
            links,
            source.handle,
            source.last_hit_by,
            None,
        ) else {
            return;
        };
        let counter_damage = scholar_scaled_amount(damage, SCHOLAR_SYMPATHY_COUNTER_RATE);
        if counter_damage <= 0 {
            return;
        }
        self.apply_scholar_popup_damage(world_chr_man, attacker, counter_damage, now);
    }

    fn apply_scholar_sympathy_heals(
        &mut self,
        world_chr_man: &mut HudWorld,
        links: &[ScholarLink],
        now: Instant,
    ) {
        for source in links.iter().filter(|link| scholar_link_is_friendly(link)) {
            let Some(previous_hp) = self.scholar_sympathy_hp.get(&source.handle).copied() else {
                continue;
            };
            let restored = source.hp - previous_hp;
            if restored <= 0 {
                continue;
            }

            let ally_heal = scholar_scaled_amount(restored, SCHOLAR_SYMPATHY_HEAL_SPREAD_RATE);
            if ally_heal > 0 {
                for ally in links
                    .iter()
                    .filter(|link| scholar_link_is_friendly(link) && link.handle != source.handle)
                {
                    apply_hp_delta_by_handle(world_chr_man, ally.handle, ally_heal);
                }
            }

            let enemy_damage = scholar_scaled_amount(restored, SCHOLAR_SYMPATHY_HEAL_DAMAGE_RATE);
            if enemy_damage > 0 {
                for enemy in links
                    .iter()
                    .filter(|link| link.kind == ScholarLinkKind::Enemy)
                {
                    self.apply_scholar_popup_damage(world_chr_man, enemy.handle, enemy_damage, now);
                }
            }
        }
    }

    fn apply_scholar_popup_damage(
        &mut self,
        world_chr_man: &mut HudWorld,
        handle: HudHandle,
        damage: i32,
        now: Instant,
    ) {
        if damage <= 0 {
            return;
        }
        if self.apply_scholar_speffect_damage(world_chr_man, handle, damage, now) {
            return;
        }
        let applied = -apply_hp_delta_by_handle(world_chr_man, handle, -damage);
        if applied <= 0 {
            return;
        }
        self.push_scholar_damage_number(world_chr_man, handle, applied, now);
    }

    fn apply_scholar_speffect_damage(
        &mut self,
        world_chr_man: &mut HudWorld,
        handle: HudHandle,
        damage: i32,
        now: Instant,
    ) -> bool {
        if set_scholar_sympathy_speffect_damage(damage)
            && apply_scholar_sympathy_damage_speffect(world_chr_man, handle)
        {
            self.add_scholar_speffect_damage_ignore(handle, now);
            true
        } else {
            false
        }
    }

    fn push_scholar_damage_number(
        &mut self,
        world_chr_man: &HudWorld,
        handle: HudHandle,
        amount: i32,
        now: Instant,
    ) {
        let Some(pos) = scholar_damage_number_pos(world_chr_man, handle) else {
            return;
        };
        if self.scholar_damage_numbers.len() >= SCHOLAR_DAMAGE_NUMBER_MAX {
            self.scholar_damage_numbers.remove(0);
        }
        let seed = (self.scholar_damage_numbers.len() % 5) as f32;
        self.scholar_damage_numbers.push(ScholarDamageNumber {
            pos,
            amount,
            created_at: now,
            seed,
        });
    }

    fn add_scholar_speffect_damage_ignore(&mut self, handle: HudHandle, now: Instant) {
        let expires_at = now + Duration::from_secs_f32(SCHOLAR_SPEFFECT_DAMAGE_IGNORE_SECONDS);
        self.scholar_native_damage_ignores
            .entry(handle)
            .and_modify(|(amount, existing_expires_at)| {
                *amount = i32::MAX / 4;
                *existing_expires_at = expires_at;
            })
            .or_insert((i32::MAX / 4, expires_at));
    }

    fn consume_scholar_native_damage_ignore(&mut self, handle: HudHandle, damage: i32) -> i32 {
        let Some((ignored, _)) = self.scholar_native_damage_ignores.get_mut(&handle) else {
            return 0;
        };
        let consumed = damage.min(*ignored);
        *ignored -= consumed;
        if *ignored <= 0 {
            self.scholar_native_damage_ignores.remove(&handle);
        }
        consumed
    }

    fn refresh_scholar_sympathy_hp(&mut self, world_chr_man: &HudWorld, links: &[ScholarLink]) {
        let mut next = HashMap::new();
        for link in links {
            if let Some(hp) = hp_by_handle(world_chr_man, link.handle) {
                next.insert(link.handle, hp);
            }
        }
        self.scholar_sympathy_hp = next;
    }

    #[allow(clippy::too_many_arguments, reason = "Keep the existing role input snapshot together")]
    fn consume_inputs(
        &mut self,
        player: &mut HudPlayer,
        config: &RoleConfig,
        active_effects: &[i32],
        locked_element: Option<RecluseElement>,
        locked_damage_module: Option<usize>,
        world_chr_man: &mut HudWorld,
        now: Instant,
    ) {
        let skill_used = consume_event_effect(player, active_effects, config, SP_SKILL_USED);
        let recluse_released = config.role == Role::Recluse
            && consume_effect(player, active_effects, config.effect(SP_RECLUSE_RELEASED));
        let _skill_used_second =
            consume_event_effect(player, active_effects, config, SP_SKILL_USED_SECOND);
        let skill_used_enhanced = consume_effect(
            player,
            active_effects,
            config.effect(SP_SKILL_USED_ENHANCED),
        );
        let skill_cancel = consume_effect(player, active_effects, config.effect(SP_SKILL_CANCEL));
        let ultimate_charged_used =
            consume_event_effect(player, active_effects, config, SP_ULTIMATE_CHARGED_USED);
        let ultimate_uncharged_used =
            consume_event_effect(player, active_effects, config, SP_ULTIMATE_UNCHARGED_USED);
        let undertaker_free_ultimate_trigger = config.role == Role::Undertaker
            && consume_effect(
                player,
                active_effects,
                config.effect(SP_UNDERTAKER_ULTIMATE_FREE_TRIGGER),
            );
        let executor_ultimate_cancel = config.role == Role::Executor
            && consume_effect(
                player,
                active_effects,
                config.effect(SP_EXECUTOR_ULTIMATE_CANCEL),
            );
        let ultimate_kill_gain =
            consume_event_effect(player, active_effects, config, SP_ULTIMATE_KILL_GAIN);
        let ultimate_critical_gain =
            consume_event_effect(player, active_effects, config, SP_ULTIMATE_CRITICAL_GAIN);
        let scholar_apply_self = config.role == Role::Scholar
            && consume_effect(player, active_effects, config.effect(SP_SCHOLAR_APPLY_SELF));
        let scholar_apply_enemy = config.role == Role::Scholar
            && consume_effect(
                player,
                active_effects,
                config.effect(SP_SCHOLAR_APPLY_ENEMY),
            );

        if scholar_apply_self {
            self.apply_scholar_self_effect(player, config);
        }
        if scholar_apply_enemy {
            self.apply_scholar_enemy_effect(world_chr_man, config);
        }
        if scholar_apply_self || scholar_apply_enemy {
            self.clear_scholar_observations();
        }
        if ultimate_kill_gain && self.executor_transform_end.is_none() && self.kill_gain_ready(now)
        {
            self.ultimate = (self.ultimate + ULTIMATE_BIG_GAIN).clamp(0.0, 1.0);
            self.last_kill_gain = now;
        }
        if ultimate_critical_gain
            && self.executor_transform_end.is_none()
            && self.critical_gain_ready(now)
        {
            self.ultimate = (self.ultimate + ULTIMATE_BIG_GAIN).clamp(0.0, 1.0);
            self.last_critical_gain = now;
        }
        if executor_ultimate_cancel && self.executor_transform_end.is_some() {
            self.executor_transform_end = None;
            self.ultimate = 0.0;
        }
        if undertaker_free_ultimate_trigger {
            self.undertaker_free_ultimate_end =
                Some(now + Duration::from_secs_f32(UNDERTAKER_FREE_ULTIMATE_SECONDS));
        }
        if recluse_released {
            self.clear_recluse_magic(player, config);
        }

        let skill_use_allowed = if config.role == Role::Recluse {
            skill_used && self.recluse_absorb_ready(now)
        } else {
            skill_used && self.skill_consume_ready(now)
        };
        let skill_enhanced_use_allowed = skill_used_enhanced && self.skill_consume_ready(now);
        if skill_use_allowed || skill_enhanced_use_allowed {
            self.last_skill_consume = now;
        }

        match config.skill_kind {
            SkillKind::NoCooldown => {
                if config.role == Role::Recluse && skill_use_allowed {
                    self.handle_recluse_skill(
                        player,
                        config,
                        locked_element,
                        locked_damage_module,
                        now,
                    );
                }
            }
            SkillKind::Timed => {
                if skill_use_allowed {
                    let had_skill_charge = self.skill_ready_count(config) > 0;
                    spend_skill_charge(&mut self.skill_layers, config.skill_charges);
                    if config.role == Role::Duchess && had_skill_charge {
                        self.handle_duchess_skill(player, world_chr_man, now);
                    }
                }
            }
            SkillKind::UndertakerBuff => {
                if self.undertaker_buff_end.is_some() {
                    if skill_use_allowed || skill_cancel {
                        self.undertaker_buff_end = None;
                        self.skill_layers = [0.0, 0.0];
                        clear_undertaker_skill_effects(player);
                    }
                } else if self.skill_layers[0] >= 1.0
                    && (skill_enhanced_use_allowed || skill_use_allowed)
                {
                    let effect = self.begin_undertaker_skill(skill_enhanced_use_allowed, now);
                    clear_undertaker_skill_effects(player);
                    native_character::report_effect_result(
                        // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                        unsafe {
                            native_character::apply_effect(
                                player.actor.identity.address,
                                effect,
                                true,
                            )
                        },
                    );
                }
            }
        }

        if ultimate_charged_used {
            if config.role == Role::Executor {
                if self.executor_transform_end.is_none() {
                    self.executor_transform_end =
                        Some(now + Duration::from_secs_f32(EXECUTOR_TRANSFORM_SECONDS));
                    self.ultimate = 1.0;
                }
            } else if config.role == Role::Undertaker && self.undertaker_free_ultimate_active(now) {
                self.undertaker_free_ultimate_end = None;
            } else {
                self.ultimate = 0.0;
            }
        }
        if ultimate_uncharged_used && config.discounted_ultimate {
            self.ultimate = (self.ultimate - config.ultimate_full_cost).clamp(0.0, 1.0);
        }
    }

    fn apply_scholar_self_effect(&mut self, player: &mut HudPlayer, config: &RoleConfig) {
        if let Some(stage) = self.best_scholar_stage() {
            clear_scholar_self_outputs(player, config);
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        player.actor.identity.address,
                        config.effect(SP_SCHOLAR_SELF_STAGE_BASE + stage as i32 - 1),
                        true,
                    )
                },
            );
        }
    }

    fn apply_scholar_enemy_effect(&mut self, world_chr_man: &mut HudWorld, config: &RoleConfig) {
        let targets = self.scholar_enemy_effect_targets();
        self.clear_scholar_enemy_effects(world_chr_man, config);
        for (handle, stage) in targets {
            let Some(mut enemy) = hud_actor_by_handle(world_chr_man, &handle) else {
                continue;
            };
            if scholar_reject_reason(&enemy).is_some() {
                continue;
            }
            clear_scholar_enemy_outputs(&mut enemy, config);
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        enemy.identity.address,
                        config.effect(SP_SCHOLAR_ENEMY_STAGE_BASE + stage as i32 - 1),
                        true,
                    )
                },
            );
            self.scholar_enemy_effect_targets.push(handle);
        }
    }

    fn scholar_enemy_effect_targets(&self) -> Vec<(HudHandle, usize)> {
        self.scholar_targets
            .iter()
            .filter_map(|(handle, target)| {
                if handle.is_empty() {
                    return None;
                }
                target.stage().map(|stage| (*handle, stage))
            })
            .collect()
    }

    fn clear_scholar_enemy_effects(&mut self, world_chr_man: &mut HudWorld, config: &RoleConfig) {
        for handle in self.scholar_enemy_effect_targets.drain(..) {
            if let Some(mut enemy) = hud_actor_by_handle(world_chr_man, &handle) {
                clear_scholar_enemy_outputs(&mut enemy, config);
            }
        }
    }

    fn clear_scholar_observations(&mut self) {
        self.scholar_targets.clear();
        self.scholar_debug = ScholarScanDebug::default();
    }

    fn best_scholar_stage(&self) -> Option<usize> {
        self.best_scholar_target()
            .and_then(|(_, target)| target.stage())
    }

    fn best_scholar_target(&self) -> Option<(HudHandle, ScholarTargetState)> {
        self.scholar_targets
            .iter()
            .filter(|(_, target)| target.progress > 0.0)
            .max_by(|(_, a), (_, b)| {
                a.progress
                    .partial_cmp(&b.progress)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(key, target)| (*key, *target))
    }

    fn handle_recluse_skill(
        &mut self,
        player: &mut HudPlayer,
        config: &RoleConfig,
        locked_element: Option<RecluseElement>,
        locked_damage_module: Option<usize>,
        now: Instant,
    ) {
        if self.recluse_ready_magic.is_some() {
            return;
        }

        let Some(element) = locked_element else {
            return;
        };

        self.push_recluse_element(element);
        self.last_recluse_absorb = now;
        if let Some(damage_module) = locked_damage_module {
            clear_recluse_damage_element(damage_module);
        }
        self.recluse_lock_element = None;
        self.recluse_lock_damage_module = None;
        self.recluse_lock_pos = None;
        self.recluse_lock_debug_points = [None; 5];
        self.recluse_ready_magic = self
            .recluse_slots_full()
            .then(|| self.recluse_magic_index())
            .flatten();
        clear_recluse_absorb_outputs(player, config);
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::apply_effect(
                    player.actor.identity.address,
                    config.effect(SP_RECLUSE_RESTORE_FP),
                    true,
                )
            },
        );
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::apply_effect(
                    player.actor.identity.address,
                    config.effect(element.absorb_effect_offset()),
                    true,
                )
            },
        );
        if let Some(magic_index) = self.recluse_ready_magic {
            clear_recluse_magic_outputs(player, config);
            native_character::report_effect_result(
                // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                unsafe {
                    native_character::apply_effect(
                        player.actor.identity.address,
                        config.effect(SP_RECLUSE_MIXED_MAGIC_BASE + magic_index as i32 - 1),
                        true,
                    )
                },
            );
        }
        self.recluse_outputs_clear_at =
            Some(now + Duration::from_secs_f32(RECLUSE_TRANSIENT_OUTPUT_SECONDS));
    }

    fn handle_duchess_skill(
        &mut self,
        player: &mut HudPlayer,
        world_chr_man: &mut HudWorld,
        now: Instant,
    ) {
        let player_pos = hud_physics_position(&player.actor);
        let mut targets = Vec::new();

        let entries = match native_world::update_entries(world_chr_man.object.address) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
                return;
            }
        };
        for entry in entries {
            if let Err(error) = native_world::revalidate(entry.character) {
                eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
                return;
            }
            // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
            // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
            let copied = match HudActor::observe(entry.character) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                    return;
                }
            };
            let enemy = &copied;
            if scholar_reject_reason(enemy).is_some() {
                continue;
            }
            let enemy_pos = scholar_chr_physics_position(enemy);
            if distance_havok(player_pos, enemy_pos) > DUCHESS_REPLAY_RADIUS {
                continue;
            }
            let damage_module = damage_module_from_chr_ins(enemy);
            let payload =
                duchess_replay_payload_for(damage_module, now).scaled(DUCHESS_REPLAY_RATE);
            if payload.has_effect() {
                targets.push((enemy.handle, damage_module, payload));
            }
        }

        for (handle, damage_module, payload) in targets {
            self.apply_duchess_replay(world_chr_man, handle, damage_module, payload);
        }
    }

    fn apply_duchess_replay(
        &mut self,
        world_chr_man: &mut HudWorld,
        handle: HudHandle,
        damage_module: usize,
        payload: DuchessReplayPayload,
    ) {
        if !payload.has_effect() {
            return;
        }
        if set_duchess_replay_speffect(payload) {
            if let Some(mut enemy) = hud_actor_by_handle(world_chr_man, &handle) {
                add_duchess_replay_ignore(damage_module, payload);
                native_character::report_effect_result(
                    // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
                    unsafe {
                        native_character::apply_effect(
                            enemy.identity.address,
                            SP_DUCHESS_REPLAY_DAMAGE,
                            true,
                        )
                    },
                );
                apply_duchess_replay_posture_damage(&mut enemy, payload);
            }
            return;
        }

        if let Some(mut enemy) = hud_actor_by_handle(world_chr_man, &handle) {
            if payload.hp_damage > 0 {
                apply_chr_hp_delta(&mut enemy, -payload.hp_damage);
            }
            add_duchess_replay_ignore(
                damage_module,
                DuchessReplayPayload {
                    hp_damage: payload.hp_damage,
                    status: [0; 7],
                    super_armor_damage: payload.super_armor_damage,
                },
            );
            apply_duchess_replay_posture_damage(&mut enemy, payload);
        }
    }

    fn clear_recluse_magic(&mut self, player: &mut HudPlayer, config: &RoleConfig) {
        self.recluse_elements = [None, None, None];
        self.recluse_ready_magic = None;
        clear_recluse_magic_outputs(player, config);
    }

    fn push_recluse_element(&mut self, element: RecluseElement) {
        if let Some(slot) = self.recluse_elements.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(element);
        }
    }

    fn recluse_slots_full(&self) -> bool {
        self.recluse_elements.iter().all(Option::is_some)
    }

    fn recluse_magic_index(&self) -> Option<usize> {
        let mask = self
            .recluse_elements
            .iter()
            .flatten()
            .fold(0, |mask, element| mask | element.mask());
        match mask {
            0b1011 => Some(1),  // Fire + Magic + Holy
            0b1110 => Some(2),  // Lightning + Fire + Holy
            0b1101 => Some(3),  // Magic + Lightning + Holy
            0b0010 => Some(4),  // Fire
            0b0001 => Some(5),  // Magic
            0b0100 => Some(6),  // Lightning
            0b1000 => Some(7),  // Holy
            0b0011 => Some(8),  // Fire + Magic
            0b0110 => Some(9),  // Lightning + Fire
            0b1010 => Some(10), // Fire + Holy
            0b0101 => Some(11), // Lightning + Magic
            0b1001 => Some(12), // Magic + Holy
            0b1100 => Some(13), // Holy + Lightning
            0b0111 => Some(14), // Magic + Lightning + Fire
            _ => None,
        }
    }

    fn clear_recluse_transient_outputs(
        &mut self,
        player: &mut HudPlayer,
        config: &RoleConfig,
        now: Instant,
    ) {
        if config.role != Role::Recluse {
            return;
        }
        if self
            .recluse_outputs_clear_at
            .is_some_and(|clear_at| (clear_at - now).as_secs_f32() <= 0.0)
        {
            clear_recluse_absorb_outputs(player, config);
            self.recluse_outputs_clear_at = None;
        }
    }

    fn skill_consume_ready(&self, now: Instant) -> bool {
        (now - self.last_skill_consume).as_secs_f32() >= SKILL_CONSUME_INTERVAL
    }

    fn recluse_absorb_ready(&self, now: Instant) -> bool {
        (now - self.last_recluse_absorb).as_secs_f32() >= RECLUSE_ABSORB_INTERVAL
    }

    fn hit_gain_ready(&self, now: Instant) -> bool {
        (now - self.last_hit_gain).as_secs_f32() >= ULTIMATE_HIT_GAIN_INTERVAL
    }

    fn kill_gain_ready(&self, now: Instant) -> bool {
        (now - self.last_kill_gain).as_secs_f32() >= ULTIMATE_KILL_GAIN_INTERVAL
    }

    fn critical_gain_ready(&self, now: Instant) -> bool {
        (now - self.last_critical_gain).as_secs_f32() >= ULTIMATE_CRITICAL_GAIN_INTERVAL
    }

    fn sync_output_speffects(&self, player: &mut HudPlayer, config: &RoleConfig) {
        let skill_ready_count = self.skill_ready_count(config);
        set_effect(
            player,
            shared_effect(SP_SKILL_READY_1),
            skill_ready_count >= 1,
        );
        set_effect(
            player,
            shared_effect(SP_SKILL_READY_2),
            config.skill_charges >= 2 && skill_ready_count >= 2,
        );

        set_effect(
            player,
            shared_effect(SP_ULTIMATE_READY),
            self.ultimate_ready(config),
        );
    }

    fn snapshot(
        &self,
        world_chr_man: &HudWorld,
        config: &RoleConfig,
        action_pressed: bool,
        now: Instant,
    ) -> HudSnapshot {
        let skill_ready_count = self.skill_ready_count(config);
        let skill_top_down =
            config.skill_kind == SkillKind::UndertakerBuff && self.undertaker_buff_end.is_some();
        let skill_count = (config.skill_charges >= 2).then_some(skill_ready_count);
        let ultimate_ready = self.ultimate_ready(config);
        let ultimate_top_down =
            config.role == Role::Executor && self.executor_transform_end.is_some();
        let undertaker_free_ultimate_active =
            config.role == Role::Undertaker && self.undertaker_free_ultimate_active(now);

        let mut skill_layers = self.skill_layers;
        if config.skill_kind == SkillKind::UndertakerBuff
            && let Some(end) = self.undertaker_buff_end {
                let remaining = (end - now).as_secs_f32();
                skill_layers = [
                    (remaining / self.undertaker_buff_duration).clamp(0.0, 1.0),
                    0.0,
                ];
            }
        let mut scholar_targets = [ScholarTargetSnapshot::default(); SCHOLAR_SCAN_MAX_TARGETS];
        let mut scholar_damage_numbers =
            [ScholarDamageNumberSnapshot::default(); SCHOLAR_DAMAGE_NUMBER_MAX];
        let mut ironeye_weakness_marks =
            [IroneyeWeaknessSnapshot::default(); IRONEYE_WEAKNESS_MAX_TARGETS];
        let mut ironeye_weakness_bursts =
            [IroneyeWeaknessBurstSnapshot::default(); IRONEYE_WEAKNESS_MAX_BURSTS];
        let mut revenant_summons = [RevenantSummonSnapshot::default(); REVENANT_SUMMON_COUNT];
        if config.role == Role::Revenant {
            for (index, summon) in revenant_summons.iter_mut().enumerate() {
                *summon = RevenantSummonSnapshot {
                    active: self.revenant_active_summon == Some(index),
                    hp_ratio: self.revenant_summon_hp[index].clamp(0.0, 1.0),
                };
            }
        }
        if config.role == Role::Ironeye {
            if let Some(player) = world_chr_man.player {
                for (slot, (handle, mark)) in ironeye_weakness_marks
                    .iter_mut()
                    .zip(self.ironeye_weakness_marks.iter())
                {
                    let Some(pos) = hud_actor_by_handle(world_chr_man, handle)
                        .and_then(|target| ironeye_weakness_marker_screen_pos(&player, &target))
                    else {
                        continue;
                    };
                    *slot = IroneyeWeaknessSnapshot {
                        active: true,
                        pos,
                        progress: (mark.accumulated_damage as f32 / mark.threshold_damage as f32)
                            .clamp(0.0, 1.0),
                        remaining: (mark.expires_at - now).as_secs_f32().max(0.0),
                    };
                }
            }
            for (slot, burst) in ironeye_weakness_bursts
                .iter_mut()
                .zip(self.ironeye_weakness_bursts.iter())
            {
                let age = (now - burst.created_at).as_secs_f32();
                if age >= IRONEYE_WEAKNESS_BURST_SECONDS {
                    continue;
                }
                *slot = IroneyeWeaknessBurstSnapshot {
                    active: true,
                    pos: burst.pos,
                    age,
                };
            }
        }
        if config.role == Role::Scholar {
            for (slot, target) in scholar_targets.iter_mut().zip(
                self.scholar_targets
                    .values()
                    .filter(|target| target.progress > 0.0),
            ) {
                *slot = ScholarTargetSnapshot {
                    active: true,
                    observed: target.visible,
                    pos: target.pos,
                    progress: target.progress,
                };
            }
            for (slot, number) in scholar_damage_numbers
                .iter_mut()
                .zip(self.scholar_damage_numbers.iter())
            {
                let age = (now - number.created_at).as_secs_f32();
                if age >= SCHOLAR_DAMAGE_NUMBER_SECONDS {
                    continue;
                }
                *slot = ScholarDamageNumberSnapshot {
                    active: true,
                    pos: number.pos,
                    amount: number.amount,
                    age,
                    seed: number.seed,
                };
            }
        }
        HudSnapshot {
            role: config.role,
            skill_layers,
            skill_ready: skill_ready_count > 0,
            skill_top_down,
            skill_count,
            wylder_skill_lock_active: config.role == Role::Wylder && self.wylder_skill_lock_active,
            wylder_skill_lock_has_target: config.role == Role::Wylder
                && self.wylder_skill_lock_has_target,
            wylder_skill_lock_pos: (config.role == Role::Wylder)
                .then_some(self.wylder_skill_lock_pos)
                .flatten(),
            wylder_skill_lock_scale: if config.role == Role::Wylder {
                self.wylder_skill_lock_scale
            } else {
                1.0
            },
            ironeye_precision_aiming: IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed),
            standalone_precision_aiming: STANDALONE_PRECISION_AIMING.load(Ordering::Relaxed),
            precision_calibration_aiming: PRECISION_CALIBRATION_AIMING.load(Ordering::Relaxed),
            ironeye_weakness_marks,
            ironeye_weakness_bursts,
            recluse_elements: if config.role == Role::Recluse {
                self.recluse_elements
            } else {
                [None, None, None]
            },
            recluse_ready_magic: (config.role == Role::Recluse)
                .then_some(self.recluse_ready_magic)
                .flatten(),
            recluse_lock_element: (config.role == Role::Recluse)
                .then_some(self.recluse_lock_element)
                .flatten(),
            recluse_lock_pos: (config.role == Role::Recluse)
                .then_some(self.recluse_lock_pos)
                .flatten(),
            recluse_lock_debug_points: if config.role == Role::Recluse {
                self.recluse_lock_debug_points
            } else {
                [None; 5]
            },
            recluse_blood_debug: self.recluse_blood_debug,
            scholar_observing: config.role == Role::Scholar && self.scholar_observing,
            scholar_targets,
            scholar_damage_numbers,
            scholar_debug: if config.role == Role::Scholar {
                self.scholar_debug
            } else {
                ScholarScanDebug::default()
            },
            revenant_summons,
            revenant_skill_move_hud: config.role == Role::Revenant && self.revenant_skill_move_hud,
            revenant_effect_mask: if config.role == Role::Revenant {
                self.revenant_effect_mask
            } else {
                0
            },
            revenant_active_summon: if config.role == Role::Revenant {
                self.revenant_active_summon
            } else {
                None
            },
            revenant_buddy_count: if config.role == Role::Revenant {
                self.revenant_buddy_count
            } else {
                0
            },
            revenant_active_handle_container: if config.role == Role::Revenant {
                self.revenant_active_summon
                    .and_then(|index| self.revenant_summon_handles[index])
                    .map(|handle| handle.diagnostic_set_index())
                    .unwrap_or(u32::MAX)
            } else {
                u32::MAX
            },
            revenant_active_handle_index: if config.role == Role::Revenant {
                self.revenant_active_summon
                    .and_then(|index| self.revenant_summon_handles[index])
                    .map(|handle| handle.diagnostic_ordinal())
                    .unwrap_or(u32::MAX)
            } else {
                u32::MAX
            },
            revenant_active_npc_id: if config.role == Role::Revenant {
                self.revenant_active_summon
                    .and_then(|index| self.revenant_summon_handles[index])
                    .and_then(|handle| hud_actor_by_handle(world_chr_man, &handle))
                    .map(|chr| chr.resource_number)
                    .unwrap_or(-1)
            } else {
                -1
            },
            undertaker_normal_skill_buff: config.role == Role::Undertaker
                && self.undertaker_buff_end.is_some()
                && self.undertaker_buff_duration < UNDERTAKER_ENHANCED_BUFF_SECONDS,
            undertaker_enhanced_skill_buff: config.role == Role::Undertaker
                && self.undertaker_buff_end.is_some()
                && self.undertaker_buff_duration >= UNDERTAKER_ENHANCED_BUFF_SECONDS,
            ultimate_progress: self.ultimate,
            ultimate_ready,
            ultimate_top_down,
            hud_solid: action_pressed,
            undertaker_free_ultimate_active,
            executor_ultimate_active: config.role == Role::Executor
                && self.executor_transform_end.is_some(),
        }
    }

    fn ultimate_ready(&self, config: &RoleConfig) -> bool {
        (config.role == Role::Undertaker && self.undertaker_free_ultimate_end.is_some())
            || (self.ultimate >= 1.0 && self.executor_transform_end.is_none())
    }

    fn undertaker_free_ultimate_active(&self, now: Instant) -> bool {
        self.undertaker_free_ultimate_end
            .is_some_and(|end| (end - now).as_secs_f32() > 0.0)
    }

    fn skill_ready_count(&self, config: &RoleConfig) -> usize {
        match config.skill_kind {
            SkillKind::NoCooldown => 1,
            SkillKind::UndertakerBuff if self.undertaker_buff_end.is_some() => 0,
            _ => self
                .skill_layers
                .iter()
                .take(config.skill_charges)
                .filter(|charge| **charge >= 1.0)
                .count(),
        }
    }
}

fn wylder_skill_lock_target(world_chr_man: &HudWorld) -> Option<WylderSkillLockTarget> {
    let player = world_chr_man.player?;
    wylder_skill_lock_enemy_target_pos(world_chr_man, &player)
        .or_else(|| wylder_skill_lock_wall_target_pos(&player))
}

fn sync_precision_calibration_outputs(world_chr_man: &mut HudWorld, active_effects: &[i32]) {
    let calibration_active = active_effects.contains(&SP_PRECISION_CALIBRATION);
    let has_target = calibration_active && precision_calibration_has_enemy_target(world_chr_man);
    let Some(player) = world_chr_man.player.as_mut() else {
        return;
    };

    if !calibration_active {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    SP_PRECISION_CALIBRATION_NO_TARGET,
                )
            },
        );
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    SP_PRECISION_CALIBRATION_TARGET,
                )
            },
        );
        return;
    }

    let desired = if has_target {
        SP_PRECISION_CALIBRATION_TARGET
    } else {
        SP_PRECISION_CALIBRATION_NO_TARGET
    };
    let undesired = if has_target {
        SP_PRECISION_CALIBRATION_NO_TARGET
    } else {
        SP_PRECISION_CALIBRATION_TARGET
    };

    if active_effects.contains(&undesired) {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, undesired) },
        );
    }
    if !active_effects.contains(&desired) {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::apply_effect(player.actor.identity.address, desired, true) },
        );
    }
}

fn precision_calibration_has_enemy_target(world_chr_man: &HudWorld) -> bool {
    let Some(player) = world_chr_man.player else {
        return false;
    };
    wylder_skill_lock_enemy_target_pos(world_chr_man, &player).is_some()
}

fn wylder_skill_lock_enemy_target_pos(
    world_chr_man: &HudWorld,
    player: &HudPlayer,
) -> Option<WylderSkillLockTarget> {
    let player_pos = hud_physics_position(&player.actor);
    let mut best: Option<(WylderSkillLockTarget, f32)> = None;
    let entries = match native_world::update_entries(world_chr_man.object.address) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
            return None;
        }
    };
    for entry in entries.into_iter().take(WYLDER_SKILL_LOCK_MAX_DISTANCE_ENTRIES) {

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
        if scholar_reject_reason(chr).is_some() {
            continue;
        }
        let target_pos = scholar_chr_physics_position(chr);
        let target_distance = distance_havok(player_pos, target_pos);
        if target_distance > WYLDER_SKILL_LOCK_MAX_DISTANCE {
            continue;
        }
        let Some(screen_pos) = scholar_target_screen_pos(chr) else {
            continue;
        };
        if !wylder_skill_lock_screen_pos_in_reticle(screen_pos) {
            continue;
        }
        if !scholar_has_line_of_sight(chr, player) {
            continue;
        }
        let score = wylder_skill_lock_screen_pos_score(screen_pos).unwrap_or(f32::MAX);
        if match best {
            Some((_, best_score)) => score < best_score,
            None => true,
        } {
            best = Some((
                WylderSkillLockTarget {
                    kind: WylderSkillLockTargetKind::Enemy(chr.handle),
                    screen_pos,
                    world_pos: target_pos,
                    distance: target_distance,
                },
                score,
            ));
        }
    }
    best.map(|(target, _)| target)
}

fn wylder_skill_lock_wall_target_pos(player: &HudPlayer) -> Option<WylderSkillLockTarget> {
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return None;
    };
    let cam = camera.pers_cam_1.as_ref();
    let forward = cam.forward();
    let len = (forward.0 * forward.0 + forward.1 * forward.1 + forward.2 * forward.2).sqrt();
    if !len.is_finite() || len < 0.001 {
        return None;
    }

    let physics = native_character::physics_shape(player.actor.identity.address)
        .map_err(|error| eprintln!("ShikiFSRS HUD wall origin rejected: {error}"))
        .ok()?;
    let [x, y, z, _] = physics.position_bits.map(f32::from_bits);
    let origin = HudPosition(x, y + (physics.height * 0.62).clamp(0.8, 2.2), z, 0.0);
    let distance = WYLDER_SKILL_LOCK_MAX_DISTANCE;
    let delta = HudDelta(
        forward.0 / len * distance,
        forward.1 / len * distance,
        forward.2 / len * distance,
    );
    let mut best: Option<WylderSkillLockTarget> = None;
    for filter in WYLDER_SKILL_LOCK_WALL_FILTERS {
        let hit = match hud_cast_ray(player, filter, origin, delta) {
            Ok(Some(hit)) => hit,
            Ok(None) => continue,
            Err(error) => {
                eprintln!(
                    "ShikiFSRS HUD line query failed or uncertain; receipt retained: {error}"
                );
                return None;
            }
        };
        let hit_distance = distance_havok(origin, hit);
        let hit_screen_pos =
            project_lock_on_position(native_camera::Vector4(hit.0, hit.1, hit.2, 1.0), cam);
        if hit_distance.is_finite()
            && hit_distance > WYLDER_SKILL_LOCK_MIN_WALL_HIT_DISTANCE
            && hit_distance <= WYLDER_SKILL_LOCK_MAX_DISTANCE
            && hit_screen_pos.is_some_and(wylder_skill_lock_screen_pos_in_reticle)
        {
            let screen_pos = hit_screen_pos.unwrap();
            if match best {
                Some(best) => hit_distance < best.distance,
                None => true,
            } {
                best = Some(WylderSkillLockTarget {
                    kind: WylderSkillLockTargetKind::Map,
                    screen_pos,
                    world_pos: hit,
                    distance: hit_distance,
                });
            }
        }
    }
    best
}

fn wylder_skill_lock_distance_scale(distance: f32) -> f32 {
    let t = ((distance - WYLDER_SKILL_LOCK_SCALE_NEAR_DISTANCE)
        / (WYLDER_SKILL_LOCK_MAX_DISTANCE - WYLDER_SKILL_LOCK_SCALE_NEAR_DISTANCE))
        .clamp(0.0, 1.0);
    lerp(WYLDER_SKILL_LOCK_SCALE_NEAR, WYLDER_SKILL_LOCK_SCALE_FAR, t)
}

fn wylder_skill_lock_screen_pos_in_reticle(pos: [f32; 2]) -> bool {
    wylder_skill_lock_screen_pos_score(pos).is_some_and(|score| {
        let Ok(window) = native_camera::HudWindow::observe() else {
            return false;
        };
        let radius = (window.screen_height as f32).min(window.screen_width as f32)
            * WYLDER_SKILL_LOCK_SCREEN_RADIUS_FACTOR;
        score <= radius * radius
    })
}

fn wylder_skill_lock_screen_pos_score(pos: [f32; 2]) -> Option<f32> {
    let Ok(window) = native_camera::HudWindow::observe() else {
        return None;
    };
    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 || !pos[0].is_finite() || !pos[1].is_finite() {
        return None;
    }
    let center = [width * 0.5, height * 0.5];
    Some(screen_distance_sq(pos, center))
}

fn role_config(role: Role) -> &'static RoleConfig {
    &ROLE_CONFIGS[role as usize]
}

fn active_role_from_effects(active_effects: &[i32]) -> Option<Role> {
    ROLE_CONFIGS
        .iter()
        .find(|config| active_effects.contains(&config.sp_effect))
        .map(|config| config.role)
}

const CAMERA_DIRECTED_MOVEMENT_EFFECT: i32 = 880951;
const CAMERA_DIRECTED_MIN_FRAME_DELTA: f32 = 0.001;
const CAMERA_DIRECTED_MAX_FRAME_DELTA: f32 = 0.25;
const CAMERA_DIRECTED_MAX_VERTICAL_DELTA: f32 = 0.045;
const CAMERA_DIRECTED_POSITION_DELTA_ENABLED: bool = false;

fn apply_camera_directed_position_delta_if_active() {
    if !CAMERA_DIRECTED_POSITION_DELTA_ENABLED {
        if let Ok(mut last) = CAMERA_DIRECTED_LAST_POSITION.lock() {
            *last = None;
        }
        return;
    }

    let Ok(mut copied_world) = HudWorld::observe() else {
        return;
    };
    let world_chr_man = &mut copied_world;
    let Some(player) = world_chr_man.player.as_mut() else {
        return;
    };
    let current_position = hud_physics_position(&player.actor);
    if !chr_has_speffect(&player.actor, CAMERA_DIRECTED_MOVEMENT_EFFECT) {
        if let Ok(mut last) = CAMERA_DIRECTED_LAST_POSITION.lock() {
            *last = None;
        }
        return;
    }

    let Some(camera_pitch) = camera_pitch_direction() else {
        return;
    };

    let Ok(mut last_position) = CAMERA_DIRECTED_LAST_POSITION.lock() else {
        return;
    };
    let Some(previous_position) = *last_position else {
        *last_position = Some(current_position);
        return;
    };

    let dx = current_position.0 - previous_position.0;
    let dy = current_position.1 - previous_position.1;
    let dz = current_position.2 - previous_position.2;
    let horizontal_len = (dx * dx + dz * dz).sqrt();
    if !current_position.0.is_finite()
        || !current_position.1.is_finite()
        || !current_position.2.is_finite()
        || !previous_position.0.is_finite()
        || !previous_position.1.is_finite()
        || !previous_position.2.is_finite()
        || !dy.is_finite()
        || !horizontal_len.is_finite()
        || !(CAMERA_DIRECTED_MIN_FRAME_DELTA..=CAMERA_DIRECTED_MAX_FRAME_DELTA).contains(&horizontal_len)
        || dy.abs() > CAMERA_DIRECTED_MAX_FRAME_DELTA
    {
        *last_position = Some(current_position);
        return;
    }

    let vertical_delta = (camera_pitch * horizontal_len).clamp(
        -CAMERA_DIRECTED_MAX_VERTICAL_DELTA,
        CAMERA_DIRECTED_MAX_VERTICAL_DELTA,
    );
    let redirected_position = HudPosition(
        current_position.0,
        previous_position.1 + vertical_delta,
        current_position.2,
        current_position.3,
    );

    // SAFETY: Host movement callback supplies the pinned actor/controller/physics graph
    // and exclusive update phase. The SDK retains both actual stores, including partial failure.
    if let Err(error) = unsafe { native_character::publish_primary_position(
        player.actor.identity,
        [redirected_position.0, redirected_position.1, redirected_position.2, redirected_position.3]
            .map(f32::to_bits),
    ) } {
        eprintln!("ShikiFSRS HUD camera movement publication rejected: {error}");
        return;
    }

    *last_position = Some(redirected_position);
}

fn camera_pitch_direction() -> Option<f32> {
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return None;
    };
    let forward = camera.pers_cam_1.forward();
    let len = (forward.0 * forward.0 + forward.1 * forward.1 + forward.2 * forward.2).sqrt();
    if !len.is_finite() || len < 0.001 {
        return None;
    }
    Some((forward.1 / len).clamp(-1.0, 1.0))
}

const REVENANT_CONVERTED_SOUL_CHANCE_PERCENT: u32 = 15;
const REVENANT_CONVERTED_SOUL_LIMIT: usize = 10;
const REVENANT_CONVERTED_SOUL_SECONDS: f32 = 60.0;
const REVENANT_CONVERTED_SOUL_TEAM_TYPE: u8 = 47;
const REVENANT_CONVERTED_SOUL_SPEFFECT: i32 = 295000;
const REVENANT_CONVERTED_SOUL_APPEAR_ANIM_ID: i32 = 1830;
const REVENANT_CONVERTED_SOUL_DISAPPEAR_ANIM_ID: i32 = 1840;
const REVENANT_CONVERTED_SOUL_END_ANIM_SECONDS: f32 = 1.0;
const REVENANT_DEBUG_SPAWN_DELAY: Duration = Duration::from_millis(100);
const REVENANT_PASSIVE_BUDDY_ROUTE_ENABLED: bool = false;
const REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID: i32 = -1;
const REVENANT_PASSIVE_BUDDY_PARAM_IDS: [u32; REVENANT_CONVERTED_SOUL_LIMIT] = [
    52950000, 52950001, 52950002, 52950003, 52950004, 52950005, 52950006, 52950007, 52950008,
    52950009,
];

#[derive(Clone, Copy)]
struct RevenantPassiveCandidate {
    handle: HudHandle,
    npc_id: i32,
    npc_param_id: i32,
    npc_think_param_id: i32,
    position: HudPosition,
}

fn revenant_passive_candidate(
    chr: &HudActor,
    player_handle: HudHandle,
) -> Option<RevenantPassiveCandidate> {
    if chr.handle.is_empty()
        || chr.handle == player_handle
        || chr.last_effect_source != player_handle
        || chr.team == REVENANT_CONVERTED_SOUL_TEAM_TYPE
        // Independent TGA CHAR_TYPE metadata labels raw5 as NPC. This remains a
        // consumer selection policy, separate from the exact primary RTTI type.
        || chr.raw_kind != 5
        || REVENANT_BUDDY_PARAM_IDS.contains(&(chr.npc_request_id as u32))
    {
        return None;
    }

    let data = native_vitals::observe(chr.identity.address)
        .map_err(|error| eprintln!("ShikiFSRS HUD passive vital copy rejected: {error}"))
        .ok()?;
    let flags = native_character::flags(chr.identity.address)
        .map_err(|error| eprintln!("ShikiFSRS HUD passive status copy rejected: {error}"))
        .ok()?;
    if data.hp.current > 0 && !flags.native_death_marked() {
        return None;
    }

    let npc_think_param_id = revenant_enemy_think_param(chr)?;
    if chr.resource_number <= 0
        || chr.resource_number == 1000
        || chr.npc_request_id <= 0
        || npc_think_param_id <= 0
    {
        return None;
    }

    Some(RevenantPassiveCandidate {
        handle: chr.handle,
        npc_id: chr.resource_number,
        npc_param_id: chr.npc_request_id,
        npc_think_param_id,
        position: hud_physics_position(chr),
    })
}

fn revenant_enemy_think_param(chr: &HudActor) -> Option<i32> {
    native_character::initializer_word(
        chr.identity.address,
        shikifsrs_elden_ring::character_parameters::CharacterInitializerWord::EnemyThinkRequest,
    )
    .unwrap_or_else(|error| {
        eprintln!("ShikiFSRS HUD Think request copy rejected: {error}");
        None
    })
    .filter(|think_param| *think_param > 0)
}

fn revenant_debug_spawn_submission_ready(
    creator_busy: bool,
    has_unbound_request: bool,
    now: Instant,
    ready_at: Instant,
) -> bool {
    !creator_busy && !has_unbound_request && now >= ready_at
}

#[cfg(test)]
fn revenant_creator_request_consumed(request_pending: bool, creator_busy: bool) -> bool {
    request_pending && !creator_busy
}

fn revenant_passive_candidate_validation(
    candidate: &RevenantPassiveCandidate,
) -> Result<(), &'static str> {
    if !(1..=9999).contains(&candidate.npc_id)
        || candidate.npc_param_id <= 0
        || candidate.npc_think_param_id <= 0
        || !candidate.position.0.is_finite()
        || !candidate.position.1.is_finite()
        || !candidate.position.2.is_finite()
    {
        return Err("invalid-fields");
    }

    if !native_parameters::named_row_exists("NpcParam", candidate.npc_param_id as u32)
        .map_err(|_| "npc-param-unavailable")?
    {
        return Err("missing-npc-param");
    }
    if !native_parameters::named_row_exists("NpcThinkParam", candidate.npc_think_param_id as u32)
        .map_err(|_| "think-param-unavailable")?
    {
        return Err("missing-think-param");
    }
    Ok(())
}

fn revenant_converted_soul_identity_should_disappear(
    character: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> bool {
    if native_world::revalidate(character).is_err() {
        return false;
    }
    let data = match native_vitals::observe(character.address) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD retirement vital copy rejected: {error}");
            // A failed observation cannot authorize retirement.
            return false;
        }
    };
    let flags = match native_character::flags(character.address) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD retirement status copy rejected: {error}");
            return false;
        }
    };
    let disabled = match native_character::debug_requests(character.address) {
        Ok((_, disabled)) => disabled,
        Err(error) => {
            eprintln!("ShikiFSRS HUD retirement debug copy rejected: {error}");
            return false;
        }
    };
    data.hp.current <= 0 || flags.native_death_marked() || disabled
}

fn revenant_passive_roll(handle: HudHandle, now: Instant) -> bool {
    let mut value = handle.passive_seed() ^ (now.elapsed().as_nanos() as u64);
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51afd7ed558ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ceb9fe1a85ec53);
    value ^= value >> 33;
    (value % 100) < REVENANT_CONVERTED_SOUL_CHANCE_PERCENT as u64
}

fn reserve_revenant_passive_buddy_slot(
    npc_param_id: i32,
    npc_think_param_id: i32,
    active_souls: &HashMap<u32, RevenantConvertedSoul>,
) -> Option<(u32, i32)> {
    use shikifsrs_elden_ring::parameter_fields::{ParameterField as F, ScalarValue as V};
    for buddy_param_id in REVENANT_PASSIVE_BUDDY_PARAM_IDS {
        if active_souls.contains_key(&buddy_param_id) {
            continue;
        }
        let trigger_effect =
            match native_parameters::read_field(F::BuddyTriggerEffect, buddy_param_id) {
                Ok(Some(V::I32(value))) if value > 0 => value,
                Ok(_) => continue,
                Err(error) => {
                    append_damage_log(&format!("Buddy PARAM read rejected: {error}"));
                    return None;
                }
            };
        let edits = [
            (F::BuddyNpc, V::I32(npc_param_id)),
            (F::BuddyThink, V::I32(npc_think_param_id)),
            (
                F::BuddyRiddenNpc,
                V::I32(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID),
            ),
            (
                F::BuddyRiddenThink,
                V::I32(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID),
            ),
        ];
        if let Err(error) = native_parameters::write_fields(buddy_param_id, &edits) {
            append_damage_log(&format!(
                "Buddy reservation failed or uncertain; owner retained: {error}"
            ));
            return None;
        }
        return Some((buddy_param_id, trigger_effect));
    }
    None
}

fn request_revenant_passive_summon(world_chr_man: &mut HudWorld, trigger_effect: i32) {
    // SAFETY: This callback retains the exact complete exclusive native summon graph.
    if let Err(error) =
        unsafe { native_buddy::request(world_chr_man.object.address, trigger_effect) }
    {
        eprintln!(
            "ShikiFSRS HUD summon request failed or uncertain; actual records retained: {error}"
        );
        return;
    }

    if let Some(player) = world_chr_man.player.as_mut() {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::apply_effect(player.actor.identity.address, trigger_effect, true)
            },
        );
    }
}

fn prepare_revenant_passive_buddy(address: u64) {
    let effects = match native_character::effects(address) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD buddy effects copy rejected: {error}");
            return;
        }
    };
    if !effects.contains(&REVENANT_CONVERTED_SOUL_SPEFFECT) {
        // SAFETY: This preparation retains the complete effect graph and exclusive native phase.
        if let Err(error) = unsafe {
            native_character::apply_effect(address, REVENANT_CONVERTED_SOUL_SPEFFECT, true)
        } {
            eprintln!("ShikiFSRS HUD buddy marker failed or uncertain; record retained: {error}");
            return;
        }
    }

    // SAFETY: The caller owns the live native character and selected stored team byte.
    if let Err(error) =
        unsafe { native_character::set_team(address, REVENANT_CONVERTED_SOUL_TEAM_TYPE) }
    {
        eprintln!("ShikiFSRS HUD buddy team write failed or uncertain; record retained: {error}");
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) = unsafe { native_character::clear_effect_source(address) } {
        eprintln!(
            "ShikiFSRS HUD effect-source clear failed or uncertain; record retained: {error}"
        );
        return;
    }
    // SAFETY: Retain exact writable request word; the latched/prior words stay read-only.
    if let Err(error) = unsafe { native_character::set_loading_requests(address, 0x80, 0x80) } {
        eprintln!("ShikiFSRS HUD loading request rejected; actual receipt retained: {error}");
        return;
    }
    // SAFETY: Original HUD policy owns the selected initialized native result byte.
    if let Err(error) = unsafe { native_character::set_group_mask_result(address, true) } {
        eprintln!("ShikiFSRS HUD group result store rejected; actual receipt retained: {error}");
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) =
        unsafe { native_character::set_presentation_marks(address, false, true, true) }
    {
        eprintln!("ShikiFSRS HUD presentation marks failed or uncertain; record retained: {error}");
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) = unsafe { native_character::set_policy_bit3(address, true) } {
        eprintln!("ShikiFSRS HUD policy bit3 failed or uncertain; record retained: {error}");
        return;
    }
    // SAFETY: Retain exact writable debug bytes and the full exclusive graph.
    if let Err(error) = unsafe {
        native_character::update_debug_requests(
            address,
            CHR_DEBUG_FORCE_LOADED,
            CHR_DEBUG_FORCE_UNLOADED
                | CHR_DEBUG_CHARACTER_DISABLED
                | CHR_DEBUG_DISABLED_UPDATES
                | CHR_DEBUG_DISABLED_HIT
                | CHR_DEBUG_DISABLED_MOVEMENT
                | CHR_DEBUG_DISABLED_SECONDARY_ACTIONS,
        )
    } {
        eprintln!("ShikiFSRS HUD buddy debug stores rejected; records retained: {error}");
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) = unsafe { native_character::opacity(address, true) } {
        eprintln!("ShikiFSRS HUD opacity writes failed or uncertain; record retained: {error}");
        return;
    }

    // SAFETY: Actual preparation owns the experimental exclusive native vital graph.
    // The SDK retains the actual conditional store before validating its postflight.
    if let Err(error) = unsafe {
        native_vitals::change(
            address,
            shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
            native_vitals::VitalChange::RecoverDead,
        )
    } {
        eprintln!("ShikiFSRS HUD buddy recovery failed or uncertain; receipt retained: {error}");
    }
}

fn prepare_revenant_debug_soul(address: u64) {
    prepare_revenant_passive_buddy(address);
    // SAFETY: Exact native action word and exclusive full graph are retained by this callback.
    if let Err(error) = unsafe { native_character::floating_gauge(address, false) } {
        eprintln!("ShikiFSRS HUD debug gauge request rejected; actual receipt retained: {error}");
    }
}

fn request_revenant_debug_soul_animation(address: u64, anim_id: i32) {
    // SAFETY: The selected debug-soul callback retains the complete animation/event graph.
    if let Err(error) = unsafe { native_character::request_animation(address, anim_id) } {
        eprintln!("ShikiFSRS HUD soul animation failed or uncertain; record retained: {error}");
    }
}

fn release_revenant_debug_soul(world_chr_man: &mut HudWorld, soul: &RevenantDebugSoul) {
    let Some(handle) = soul.handle else {
        return;
    };
    let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) else {
        return;
    };

    // SAFETY: Actual retirement owns the experimental writable character/vital graph.
    if let Err(error) = unsafe {
        native_vitals::change(
            chr.address,
            shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
            native_vitals::VitalChange::Value(0),
        )
    } {
        eprintln!(
            "ShikiFSRS HUD soul health retirement failed or uncertain; receipt retained: {error}"
        );
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) =
        unsafe { native_character::set_presentation_marks(chr.address, true, false, false) }
    {
        eprintln!("ShikiFSRS HUD presentation marks failed or uncertain; record retained: {error}");
        return;
    }
    // SAFETY: Original selected retirement callback retains exact native writable bytes/graph.
    if let Err(error) = unsafe {
        native_character::update_debug_requests(
            chr.address,
            CHR_DEBUG_FORCE_UNLOADED
                | CHR_DEBUG_CHARACTER_DISABLED
                | CHR_DEBUG_DISABLED_UPDATES
                | CHR_DEBUG_DISABLED_HIT
                | CHR_DEBUG_DISABLED_MOVEMENT
                | CHR_DEBUG_DISABLED_SECONDARY_ACTIONS,
            CHR_DEBUG_FORCE_LOADED,
        )
    } {
        eprintln!("ShikiFSRS HUD soul retirement request failed; records retained: {error}");
        return;
    }
    // SAFETY: This callback retains the experimental exclusive native character graph.
    if let Err(error) = unsafe { native_character::opacity(chr.address, false) } {
        eprintln!("ShikiFSRS HUD opacity writes failed or uncertain; record retained: {error}");
    }
}

fn release_revenant_passive_soul(world_chr_man: &mut HudWorld, soul: &RevenantConvertedSoul) {
    clear_revenant_passive_buddy_slot(soul.buddy_param_id);

    if let Some(group) = revenant_buddy_group_by_param_id(world_chr_man, soul.buddy_param_id) {
        // SAFETY: This callback retains the experimental exclusive buddy graph.
        if let Err(error) = unsafe {
            native_vitals::change(
                group.character.address,
                shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
                native_vitals::VitalChange::Value(0),
            )
        } {
            eprintln!(
                "ShikiFSRS HUD buddy health retirement failed or uncertain; receipt retained: {error}"
            );
            return;
        }
        // SAFETY: This callback retains the experimental exclusive native character graph.
        if let Err(error) = unsafe {
            native_character::set_presentation_marks(group.character.address, true, false, false)
        } {
            eprintln!(
                "ShikiFSRS HUD presentation marks failed or uncertain; record retained: {error}"
            );
            return;
        }
        // SAFETY: Retain selected initialized writable debug bytes and exact full graph.
        if let Err(error) = unsafe {
            native_character::update_debug_requests(
                group.character.address,
                CHR_DEBUG_FORCE_UNLOADED
                    | CHR_DEBUG_CHARACTER_DISABLED
                    | CHR_DEBUG_DISABLED_UPDATES,
                CHR_DEBUG_FORCE_LOADED,
            )
        } {
            eprintln!("ShikiFSRS HUD buddy retirement stores rejected; records retained: {error}");
        }
    } else if let Some(handle) = soul.handle
        && let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
            // SAFETY: This callback retains the same experimental exclusive native graph.
            if let Err(error) = unsafe {
                native_vitals::change(
                    chr.address,
                    shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
                    native_vitals::VitalChange::Value(0),
                )
            } {
                eprintln!(
                    "ShikiFSRS HUD handle health retirement failed or uncertain; receipt retained: {error}"
                );
                return;
            }
            // SAFETY: This callback retains the experimental exclusive native character graph.
            if let Err(error) =
                unsafe { native_character::set_presentation_marks(chr.address, true, false, false) }
            {
                eprintln!(
                    "ShikiFSRS HUD presentation marks failed or uncertain; record retained: {error}"
                );
                return;
            }
            // SAFETY: Same exact initialized writable debug bytes and retained full graph.
            if let Err(error) = unsafe {
                native_character::update_debug_requests(
                    chr.address,
                    CHR_DEBUG_FORCE_UNLOADED
                        | CHR_DEBUG_CHARACTER_DISABLED
                        | CHR_DEBUG_DISABLED_UPDATES,
                    CHR_DEBUG_FORCE_LOADED,
                )
            } {
                eprintln!(
                    "ShikiFSRS HUD retirement debug stores rejected; records retained: {error}"
                );
            }
        }
}

fn clear_revenant_passive_buddy_slot(buddy_param_id: u32) {
    if let Err(error) = native_parameters::write_fields(
        buddy_param_id,
        &native_parameters::buddy_fields(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID),
    ) {
        append_damage_log(&format!(
            "Buddy release failed or uncertain; owner retained: {error}"
        ));
    }
}

fn hide_revenant_native_summon_hud_if_revenant() {
    let player = match native_world::primary_selection() {
        Ok(Some(selection)) => selection.player,
        Ok(None) => return,
        Err(error) => {
            eprintln!("ShikiFSRS HUD summon-HUD primary selection rejected: {error}");
            return;
        }
    };
    let active_effects = match native_character::effects(player.address) {
        Ok(effects) => effects,
        Err(error) => {
            eprintln!("ShikiFSRS HUD summon-HUD role observation rejected: {error}");
            return;
        }
    };
    if active_role_from_effects(&active_effects) != Some(Role::Revenant) {
        return;
    }
    if let Err(error) = native_world::revalidate(player) {
        eprintln!("ShikiFSRS HUD summon-HUD player changed: {error}");
        return;
    }
    let handles = revenant_alive_buddy_handles_at(player.world_address);
    suppress_revenant_buddy_floating_gauges(player.world_address);
    hide_revenant_native_summon_hud(&handles);
}

fn suppress_revenant_buddy_floating_gauges(world: u64) {
    let snapshot = match native_buddy::groups(world) {
        Ok(Some(value)) => value,
        Ok(None) => return,
        Err(error) => {
            eprintln!("ShikiFSRS HUD gauge buddy graph rejected: {error}");
            return;
        }
    };
    for member in snapshot.groups.iter().flat_map(|group| &group.members) {
        if !REVENANT_BUDDY_PARAM_IDS.contains(&(member.buddy_param_id as u32)) {
            continue;
        }
        let Some(target) = member.character else {
            continue;
        };
        if HudHandle::from_native(target.native_identifier).is_empty() {
            continue;
        }
        if let Err(error) = native_world::revalidate(target) {
            eprintln!("ShikiFSRS HUD gauge buddy identity rejected: {error}");
            return;
        }
        // SAFETY: Caller retains original initialized word and exact full exclusive graph.
        if let Err(error) = unsafe { native_character::floating_gauge(target.address, true) } {
            eprintln!("ShikiFSRS HUD gauge store rejected; actual receipt retained: {error}");
            return;
        }
    }
}

fn hide_revenant_native_summon_hud(revenant_buddy_handles: &[HudHandle]) {
    let handles: Vec<u64> = revenant_buddy_handles
        .iter()
        .map(|handle| {
            // Transitional owned value conversion only; never a native object or invented layout.
            handle.raw()
        })
        .collect();
    // SAFETY: Actual callback must provide the experimental exclusive frontend graph lifetime.
    if let Err(error) = unsafe { native_hud::suppress(&handles) } {
        eprintln!("Shiki revenant native HUD: {error}");
    }
}

fn revenant_alive_buddy_handles(world: &HudWorld) -> Vec<HudHandle> {
    revenant_alive_buddy_handles_at(world.object.address)
}

fn revenant_alive_buddy_handles_at(world: u64) -> Vec<HudHandle> {
    let mut handles = revenant_buddy_group_handles_at(world);
    let members = match native_world::collection(
        world,
        shikifsrs_elden_ring::character_sets::WorldCollectionSelection::Embedded10f90,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD alive-buddy collection rejected: {error}");
            return handles;
        }
    };
    for target in members {
        if revenant_buddy_hp_ratio_identity(target).is_none() {
            continue;
        }
        let handle = HudHandle::from_native(target.native_identifier);
        if !handles.contains(&handle) {
            handles.push(handle);
        }
    }
    handles
}

fn revenant_buddy_group_handles(world: &HudWorld) -> Vec<HudHandle> {
    revenant_buddy_group_handles_at(world.object.address)
}

fn revenant_buddy_group_handles_at(world: u64) -> Vec<HudHandle> {
    match native_buddy::groups(world) {
        Ok(Some(snapshot)) => native_buddy::identities(&snapshot)
            .into_iter()
            .map(|target| HudHandle::from_native(target.native_identifier))
            .collect(),
        Ok(None) => Vec::new(),
        Err(error) => {
            eprintln!("ShikiFSRS HUD buddy graph copy rejected: {error}");
            Vec::new()
        }
    }
}

fn revenant_buddy_hp_ratio_by_slot(
    world_chr_man: &HudWorld,
    index: usize,
    handle: Option<HudHandle>,
) -> Option<f32> {
    revenant_buddy_group_by_param_id(world_chr_man, REVENANT_BUDDY_PARAM_IDS[index])
        .and_then(|group| revenant_buddy_hp_ratio_identity(group.character))
        .or_else(|| {
            handle.and_then(|handle| revenant_buddy_hp_ratio_by_handle(world_chr_man, handle))
        })
}

struct RevenantBuddyGroupCharacter {
    character: shikifsrs_elden_ring::character_sets::CharacterIdentity,
    retirement_requested: bool,
}
fn revenant_buddy_group_by_param_id(
    world_chr_man: &HudWorld,
    buddy_param_id: u32,
) -> Option<RevenantBuddyGroupCharacter> {
    let snapshot = native_buddy::groups(world_chr_man.object.address)
        .map_err(|error| eprintln!("ShikiFSRS HUD buddy graph rejected: {error}"))
        .ok()??;
    let member = native_buddy::by_param_id(&snapshot, buddy_param_id)?;
    let character = member.character?;
    native_world::revalidate(character)
        .map_err(|error| eprintln!("ShikiFSRS HUD buddy identity changed: {error}"))
        .ok()?;
    Some(RevenantBuddyGroupCharacter {
        character,
        retirement_requested: member.inactive_request_byte != 0,
    })
}

fn revenant_buddy_chr_by_handle_mut(
    world: &mut HudWorld,
    handle: HudHandle,
) -> Option<shikifsrs_elden_ring::character_sets::CharacterIdentity> {
    revenant_buddy_chr_by_handle(world, handle)
}

fn revenant_buddy_chr_by_handle(
    world: &HudWorld,
    handle: HudHandle,
) -> Option<shikifsrs_elden_ring::character_sets::CharacterIdentity> {
    let world = world.object.address;
    let members = match native_world::collection(
        world,
        shikifsrs_elden_ring::character_sets::WorldCollectionSelection::Embedded10f90,
    ) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD buddy collection rejected: {error}");
            return None;
        }
    };
    if let Some(target) = members
        .into_iter()
        .find(|member| member.native_identifier == handle.raw())
    {
        return Some(target);
    }
    // SAFETY: The callback retains the exact full world/area/character graph and exclusive phase.
    unsafe { native_world::resolve(world, handle.raw()) }
        .map_err(|error| {
            eprintln!("ShikiFSRS HUD buddy lookup rejected; actual attempt retained: {error}")
        })
        .ok()
        .flatten()
}

fn revenant_debug_chr_by_handle_mut(
    world: &mut HudWorld,
    handle: HudHandle,
) -> Option<shikifsrs_elden_ring::character_sets::CharacterIdentity> {
    revenant_debug_chr_by_handle(world, handle)
}
fn revenant_debug_chr_by_handle(
    world: &HudWorld,
    handle: HudHandle,
) -> Option<shikifsrs_elden_ring::character_sets::CharacterIdentity> {
    let world = world.object.address;
    match native_creation::owned_by_handle(world, handle.raw()) {
        Ok(Some(character)) => Some(character),
        Ok(None) => {
            // SAFETY: Forward the same complete retained world/area graph and native callback phase.
            unsafe { native_world::resolve(world, handle.raw()) }
                .map_err(|error| {
                    eprintln!("ShikiFSRS HUD debug lookup failed; receipt retained: {error}")
                })
                .ok()
                .flatten()
        }
        Err(error) => {
            eprintln!("ShikiFSRS HUD debug registration superseded: {error}");
            None
        }
    }
}

fn apply_revenant_buddy_hp_ratio_address(address: u64, hp_ratio: f32) {
    // SAFETY: Actual callback supplies the exact writable native HP destination and lifetime.
    if let Err(error) = unsafe {
        native_vitals::change(
            address,
            shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
            native_vitals::VitalChange::Fraction(hp_ratio),
        )
    } {
        append_damage_log(&format!(
            "Buddy HP ratio rejected or uncertain; receipt retained: {error}"
        ));
    }
}

fn revenant_buddy_param_npc_ids() -> [Option<i32>; REVENANT_SUMMON_COUNT] {
    use shikifsrs_elden_ring::parameter_fields::{ParameterField as F, ScalarValue as V};
    REVENANT_BUDDY_PARAM_IDS.map(|id| match native_parameters::read_field(F::BuddyNpc, id) {
        Ok(Some(V::I32(value))) if value > 0 => Some(value),
        Ok(_) => None,
        Err(error) => {
            append_damage_log(&format!("Buddy NPC read rejected: {error}"));
            None
        }
    })
}

fn revenant_find_new_buddy_handle(
    world_chr_man: &HudWorld,
    target_npc_id: Option<i32>,
    current_handles: &[HudHandle],
    baseline: &[HudHandle],
    handles: &[Option<HudHandle>; REVENANT_SUMMON_COUNT],
) -> Option<HudHandle> {
    if let Some(handle) =
        revenant_find_matching_buddy_handle(world_chr_man, current_handles, target_npc_id, handles)
    {
        return Some(handle);
    }

    current_handles
        .iter()
        .copied()
        .find(|handle| {
            !baseline.iter().any(|known| known == handle)
                && !handles.iter().flatten().any(|known| known == handle)
        })
        .or_else(|| {
            revenant_find_untracked_buddy_handle(
                world_chr_man,
                target_npc_id,
                current_handles,
                handles,
            )
        })
}

fn revenant_find_untracked_buddy_handle(
    world_chr_man: &HudWorld,
    target_npc_id: Option<i32>,
    current_handles: &[HudHandle],
    handles: &[Option<HudHandle>; REVENANT_SUMMON_COUNT],
) -> Option<HudHandle> {
    if let Some(handle) =
        revenant_find_matching_buddy_handle(world_chr_man, current_handles, target_npc_id, handles)
    {
        return Some(handle);
    }

    current_handles
        .iter()
        .copied()
        .find(|handle| !handles.iter().flatten().any(|known| known == handle))
        .or_else(|| current_handles.first().copied())
}

fn revenant_find_matching_buddy_handle(
    world: &HudWorld,
    current_handles: &[HudHandle],
    target_npc_id: Option<i32>,
    handles: &[Option<HudHandle>; REVENANT_SUMMON_COUNT],
) -> Option<HudHandle> {
    let expected = target_npc_id?;
    current_handles.iter().copied().find(|handle| {
        if handles.iter().flatten().any(|known| known == handle) {
            return false;
        }
        let Some(target) = revenant_buddy_chr_by_handle(world, *handle) else {
            return false;
        };
        native_character::initializer_word(
            target.address,
            shikifsrs_elden_ring::character_parameters::CharacterInitializerWord::ResourceNumber,
        )
        .map_err(|error| eprintln!("ShikiFSRS HUD buddy resource-number rejected: {error}"))
        .is_ok_and(|value| value == Some(expected))
    })
}

fn revenant_buddy_hp_ratio_by_handle(world: &HudWorld, handle: HudHandle) -> Option<f32> {
    revenant_buddy_hp_ratio_from_fe(handle).or_else(|| {
        revenant_buddy_chr_by_handle(world, handle).and_then(revenant_buddy_hp_ratio_identity)
    })
}

fn revenant_active_effect_index(
    active_index: Option<usize>,
    pending_index: Option<usize>,
    hp_ratio: Option<f32>,
) -> Option<usize> {
    let index = active_index.filter(|index| *index < REVENANT_SUMMON_COUNT)?;
    if pending_index == Some(index) || hp_ratio? <= 0.0 {
        return None;
    }
    Some(index)
}

fn revenant_buddy_hp_ratio_from_fe(handle: HudHandle) -> Option<f32> {
    if handle.is_empty() {
        return None;
    }
    native_hud::buddy_hp_ratio(handle.raw()).unwrap_or_else(|error| {
        eprintln!("ShikiFSRS HUD buddy gauge copy rejected: {error}");
        None
    })
}

fn collect_resurrection_accumulation_teammate_snapshots(
    snapshots: &mut Vec<TeammateResurrectionAccumulationSnapshot>,
    world_chr_man: &HudWorld,
    player_handle: HudHandle,
    player_team_type: u8,
    player_position: HudPosition,
) {
    let Ok(candidates) = native_world::collection(
        world_chr_man.object.address,
        shikifsrs_elden_ring::character_sets::WorldCollectionSelection::Players,
    ) else {
        return;
    };
    snapshots.extend(
        candidates.into_iter().filter_map(|identity| {
                let candidate = HudActor::observe(identity).ok()?;
                let chr = &candidate;
                let friendly = resurrection_accumulation_teammate_is_friendly(
                    chr.raw_kind,
                    chr.team,
                    player_team_type,
                );
                let runtime_visible = !runtime_chr_force_unloaded(chr)
                    && !runtime_chr_character_disabled(chr)
                    && native_character::load_requests(chr.identity.address)
                        .is_ok_and(|value| !value.backread_disabled_requested());
                if !resurrection_accumulation_teammate_should_display(
                    chr.handle.is_empty(),
                    chr.handle == player_handle,
                    friendly,
                    runtime_visible,
                    chr_has_speffect(chr, SP_RESURRECTION_ACCUMULATION_HUD),
                ) {
                    return None;
                }

                // SAFETY: Same exclusive native graph obligation as the owning HUD callback.
                let (current, maximum) =
                    unsafe { native_vitals::super_armor_values(chr.identity.address) }
                        .map_err(|error| {
                            eprintln!("ShikiFSRS HUD teammate posture query rejected: {error}")
                        })
                        .ok()?;
                let screen_pos =
                    resurrection_accumulation_teammate_screen_pos(player_position, chr)?;
                Some(TeammateResurrectionAccumulationSnapshot {
                    progress: normalized_super_armor(current, maximum),
                    screen_pos,
                })
            }),
    )
}

fn resurrection_accumulation_teammate_is_friendly(
    raw_kind: i32,
    candidate_team_type: u8,
    player_team_type: u8,
) -> bool {
    // Host/white/blue: independent ER EMEDF TargetType 0/1/17. The pinned TGA
    // role metadata also labels 8/10/11/12/13 (hollow/transparent/invisible/
    // dragon/arena). Only these ordinary non-invader kinds use same-team admission.
    // Keep unnamed values closed; this consumer policy does not name SDK enum variants,
    // prove a network role, or turn the cooperative NPC records into human teammates.
    matches!(raw_kind, 0 | 1 | 17)
        || (matches!(raw_kind, 8 | 10 | 11 | 12 | 13)
            && player_team_type != 0
            && candidate_team_type == player_team_type)
}

fn resurrection_accumulation_teammate_should_display(
    empty_handle: bool,
    local_player: bool,
    friendly: bool,
    runtime_visible: bool,
    has_activation_effect: bool,
) -> bool {
    !empty_handle && !local_player && friendly && runtime_visible && has_activation_effect
}

fn normalized_super_armor(current: f32, maximum: f32) -> f32 {
    if !current.is_finite() || !maximum.is_finite() || maximum <= 0.0 {
        return 0.0;
    }
    (current / maximum).clamp(0.0, 1.0)
}

fn revenant_buddy_hp_ratio_identity(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Option<f32> {
    native_world::revalidate(target)
        .map_err(|error| eprintln!("ShikiFSRS HUD buddy identity rejected: {error}"))
        .ok()?;
    let handle = HudHandle::from_native(target.native_identifier);
    if handle.is_empty() {
        return None;
    }
    revenant_buddy_hp_ratio_from_fe(handle).or_else(|| {
        let data = native_vitals::observe(target.address)
            .map_err(|error| eprintln!("ShikiFSRS HUD buddy vital copy rejected: {error}"))
            .ok()?
            .hp;
        if data.maximum <= 0 || data.current < 0 {
            return None;
        }
        Some((data.current as f32 / data.maximum as f32).clamp(0.0, 1.0))
    })
}
fn collect_scholar_links(
    world_chr_man: &HudWorld,
    config: &RoleConfig,
    active_effects: &[i32],
) -> Vec<ScholarLink> {
    let mut links = Vec::new();
    if let Some(player) = world_chr_man.player
        && active_effects.contains(&config.effect(SP_SCHOLAR_SYMPATHY_SELF)) {
            push_scholar_link(&mut links, &player.actor, ScholarLinkKind::SelfLink);
        }

    let entries = match native_world::update_entries(world_chr_man.object.address) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
            return Vec::new();
        }
    };
    for entry in entries {
        if let Err(error) = native_world::revalidate(entry.character) {
            eprintln!("ShikiFSRS HUD world candidate superseded: {error}");
            return Vec::new();
        }
        // SAFETY: Transitional native view still relies on this callback’s full graph/phase contract.
        // The copied full identity was rechecked; it grants no lease. Remaining fields are being ported.
        let copied = match HudActor::observe(entry.character) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("ShikiFSRS HUD actor copy rejected: {error}");
                return Vec::new();
            }
        };
        let chr = &copied;
        if chr_data_hp(chr) <= 0 {
            continue;
        }
        let kind = if chr_has_speffect(chr, config.effect(SP_SCHOLAR_SYMPATHY_ENEMY)) {
            ScholarLinkKind::Enemy
        } else if chr_has_speffect(chr, config.effect(SP_SCHOLAR_SYMPATHY_ALLY)) {
            ScholarLinkKind::Ally
        } else if chr_has_speffect(chr, config.effect(SP_SCHOLAR_SYMPATHY_SELF)) {
            ScholarLinkKind::SelfLink
        } else {
            continue;
        };
        push_scholar_link(&mut links, chr, kind);
    }

    links
}

fn push_scholar_link(links: &mut Vec<ScholarLink>, chr: &HudActor, kind: ScholarLinkKind) {
    if links.iter().any(|link| link.handle == chr.handle) {
        return;
    }
    links.push(ScholarLink {
        handle: chr.handle,
        kind,
        hp: chr_data_hp(chr),
        last_hit_by: (!chr.last_effect_source.is_empty()).then_some(chr.last_effect_source),
    });
}

fn scholar_link_is_friendly(link: &ScholarLink) -> bool {
    matches!(link.kind, ScholarLinkKind::SelfLink | ScholarLinkKind::Ally)
}

fn scholar_counter_attacker(
    world_chr_man: &HudWorld,
    links: &[ScholarLink],
    victim: HudHandle,
    last_hit_by: Option<HudHandle>,
    locked_on_enemy: Option<HudHandle>,
) -> Option<HudHandle> {
    if let Some(attacker) = last_hit_by
        && scholar_linked_enemy(links, attacker) {
            return Some(attacker);
        }
    if let Some(locked_on_enemy) = locked_on_enemy
        && scholar_linked_enemy(links, locked_on_enemy) {
            return Some(locked_on_enemy);
        }
    if let Some(nearest) = scholar_nearest_linked_enemy(world_chr_man, links, victim) {
        return Some(nearest);
    }

    let mut enemies = links
        .iter()
        .filter(|link| link.kind == ScholarLinkKind::Enemy)
        .map(|link| link.handle);
    let only_enemy = enemies.next()?;
    enemies.next().is_none().then_some(only_enemy)
}

fn scholar_nearest_linked_enemy(
    world_chr_man: &HudWorld,
    links: &[ScholarLink],
    victim: HudHandle,
) -> Option<HudHandle> {
    let victim_pos = scholar_chr_position_by_handle(world_chr_man, victim)?;
    links
        .iter()
        .filter(|link| link.kind == ScholarLinkKind::Enemy)
        .filter_map(|link| {
            let pos = scholar_chr_position_by_handle(world_chr_man, link.handle)?;
            Some((link.handle, distance3(victim_pos, pos)))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(handle, _)| handle)
}

fn hud_model_translation(address: u64) -> Option<native_camera::Vector4> {
    let [x, y, z, w] = native_character::model_translation(address)
        .map_err(|error| eprintln!("ShikiFSRS HUD model translation rejected: {error}"))
        .ok()?
        .map(f32::from_bits);
    Some(native_camera::Vector4(x, y, z, w))
}

fn scholar_chr_position_by_handle(
    world_chr_man: &HudWorld,
    handle: HudHandle,
) -> Option<native_camera::Vector4> {
    if let Some(player) = world_chr_man.player
        && player.actor.handle == handle {
            return hud_model_translation(player.actor.identity.address);
        }
    // SAFETY: This experimental HUD callback retains the full native lookup graph and phase.
    let target = unsafe { native_world::resolve(world_chr_man.object.address, handle.raw()) }
        .map_err(|error| eprintln!("ShikiFSRS HUD model target lookup rejected: {error}"))
        .ok()??;
    hud_model_translation(target.address)
}

fn scholar_linked_enemy(links: &[ScholarLink], handle: HudHandle) -> bool {
    links
        .iter()
        .any(|link| link.kind == ScholarLinkKind::Enemy && link.handle == handle)
}

fn scholar_scaled_amount(amount: i32, rate: f32) -> i32 {
    ((amount.max(0) as f32) * rate).floor() as i32
}

fn chr_has_speffect(chr: &HudActor, sp_effect: i32) -> bool {
    native_character::effects(chr.identity.address).map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD effect observation rejected: {error}");
            false
        },
        |effects| effects.contains(&sp_effect),
    )
}

fn wylder_hookshot_target_result(
    world_chr_man: &HudWorld,
) -> Option<(HudHandle, WylderHookshotNativeResult)> {
    let entries = match native_world::update_entries(world_chr_man.object.address) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("ShikiFSRS HUD world candidates rejected: {error}");
            return None;
        }
    };
    for entry in entries {
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
        let result = if chr_has_speffect(chr, SP_WYLDER_HOOKSHOT_IMPACT_ACCEPTED) {
            WylderHookshotNativeResult::PullEnemy
        } else if chr_has_speffect(chr, SP_WYLDER_HOOKSHOT_IMPACT_REJECTED) {
            WylderHookshotNativeResult::PullToEnemy
        } else {
            continue;
        };
        return Some((chr.handle, result));
    }
    None
}

fn wylder_hookshot_destination_sample(
    world_chr_man: &HudWorld,
    destination: WylderHookshotMovementDestination,
) -> Option<(HudPosition, Option<WylderHookshotCapsule>)> {
    match destination {
        WylderHookshotMovementDestination::Enemy(handle) => {
            hud_actor_by_handle(world_chr_man, &handle).map(|target| {
                (
                    scholar_chr_physics_position(&target),
                    wylder_hookshot_chr_capsule(&target),
                )
            })
        }
        WylderHookshotMovementDestination::Point(position) => Some((position, None)),
    }
}

fn clear_wylder_hookshot_player_outputs(world_chr_man: &mut HudWorld) {
    let Some(player) = world_chr_man.player.as_mut() else {
        return;
    };
    for output in [
        SP_WYLDER_HOOKSHOT_PULL_ENEMY,
        SP_WYLDER_HOOKSHOT_PULL_TO_ENEMY,
    ] {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, output) },
        );
    }
}

fn chr_data_hp(chr: &HudActor) -> i32 {
    native_vitals::observe(chr.identity.address).map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD health observation rejected: {error}");
            0
        },
        |vitals| vitals.hp.current,
    )
}

fn duchess_replay_snapshot(chr: &HudActor) -> DuchessReplaySnapshot {
    DuchessReplaySnapshot {
        hp: chr_data_hp(chr),
        status: duchess_resist_values(chr),
        super_armor: native_character::posture_current(chr.identity.address).unwrap_or_else(|error| {
                eprintln!("ShikiFSRS HUD replay posture observation rejected: {error}");
                0.0
            }),
    }
}

fn duchess_resist_values(chr: &HudActor) -> [i32; 7] {
    native_character::resistance(chr.identity.address).map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD replay resistance observation rejected: {error}");
            [0; 7]
        },
        |words| {
            words.map(|value| {
                if (0..1_000_000).contains(&value) {
                    value
                } else {
                    0
                }
            })
        },
    )
}

fn hp_by_handle(world_chr_man: &HudWorld, handle: HudHandle) -> Option<i32> {
    if let Some(player) = world_chr_man.player
        && player.actor.handle == handle {
            return Some(chr_data_hp(&player.actor));
        }
    // SAFETY: This experimental HUD callback retains the complete native lookup graph and phase.
    let target = unsafe { native_world::resolve(world_chr_man.object.address, handle.raw()) }
        .map_err(|error| eprintln!("ShikiFSRS HUD health target lookup rejected: {error}"))
        .ok()??;
    native_vitals::observe(target.address)
        .map_err(|error| eprintln!("ShikiFSRS HUD health target observation rejected: {error}"))
        .ok()
        .map(|value| value.hp.current)
}

fn apply_scholar_sympathy_damage_speffect(world_chr_man: &mut HudWorld, handle: HudHandle) -> bool {
    if handle.is_empty() {
        return false;
    }
    let Some(target) = hud_actor_by_handle(world_chr_man, &handle) else {
        return false;
    };
    native_character::report_effect_result(
        // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
        unsafe {
            native_character::apply_effect(
                target.identity.address,
                SP_SCHOLAR_SYMPATHY_DAMAGE,
                true,
            )
        },
    );
    true
}

fn set_scholar_sympathy_speffect_damage(damage: i32) -> bool {
    if damage <= 0 {
        return false;
    }
    native_parameters::write_fields(
        SP_SCHOLAR_SYMPATHY_DAMAGE as u32,
        &[(
            shikifsrs_elden_ring::parameter_fields::ParameterField::EffectHpDelta,
            shikifsrs_elden_ring::parameter_fields::ScalarValue::I32(damage),
        )],
    )
    .is_ok()
}

fn set_duchess_replay_speffect(payload: DuchessReplayPayload) -> bool {
    if !payload.has_effect() {
        return false;
    }
    native_parameters::write_fields(
        SP_DUCHESS_REPLAY_DAMAGE as u32,
        &native_parameters::replay_fields(payload.hp_damage, payload.status),
    )
    .is_ok()
}

fn patch_revenant_buddy_stones_once() {
    static APPLIED: AtomicBool = AtomicBool::new(false);

    if APPLIED.swap(true, Ordering::Relaxed) {
        return;
    }

    let result = (|| -> shikifsrs_runtime::Result<usize> {
        let rows = native_parameters::stone_rows()?;
        if rows.is_empty() {
            return Err(shikifsrs_runtime::Error::NativeOperationRejected(
                "BuddyStone table has no rows",
            ));
        }
        for id in &rows {
            native_parameters::write_fields(*id, &native_parameters::stone_fields())?;
        }
        for id in REVENANT_PASSIVE_BUDDY_PARAM_IDS {
            if native_parameters::row_exists(
                shikifsrs_elden_ring::parameter_fields::ParameterField::BuddyNpc,
                id,
            )? {
                native_parameters::write_fields(
                    id,
                    &native_parameters::buddy_fields(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID),
                )?;
            }
        }
        Ok(rows.len())
    })();
    match result {
        Ok(row_count) => append_damage_log(&format!(
            "[revenant] patched BuddyStoneParam summon restriction fields rows={row_count}"
        )),
        Err(error) => {
            APPLIED.store(false, Ordering::Relaxed);
            append_damage_log(&format!("[revenant] retained parameter failure: {error}"));
        }
    }
}

fn apply_duchess_replay_posture_damage(chr: &mut HudActor, payload: DuchessReplayPayload) {
    // SAFETY: Actual callback must retain both exclusive live posture fields.
    if let Err(error) = unsafe {
        native_vitals::damage_posture(chr.identity.address, payload.super_armor_damage.max(0.0))
    } {
        append_damage_log(&format!(
            "Replay posture change rejected or uncertain; receipt retained: {error}"
        ));
    }
}

fn scholar_damage_number_pos(world_chr_man: &HudWorld, handle: HudHandle) -> Option<[f32; 2]> {
    hud_actor_by_handle(world_chr_man, &handle).and_then(|actor| scholar_target_screen_pos(&actor))
}

fn fixed_fp_restore_for_player(world_chr_man: &HudWorld, accumulator: &mut f32) -> i32 {
    let Some(player) = world_chr_man.player else {
        return 0;
    };
    let vitals = match native_vitals::observe(player.actor.identity.address) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ShikiFSRS HUD FP restore vital copy rejected: {error}");
            return 0;
        }
    };
    let max_fp = vitals.mp.maximum.max(1) as f32;
    *accumulator += max_fp * RECLUSE_BLOOD_SOUL_FIXED_FP_RESTORE_RATE;
    let restore = accumulator.floor() as i32;
    if restore > 0 {
        *accumulator -= restore as f32;
    }
    restore
}

fn apply_hp_delta_by_handle(world_chr_man: &mut HudWorld, handle: HudHandle, delta: i32) -> i32 {
    if delta == 0 {
        return 0;
    }
    if let Some(player) = world_chr_man.player.as_mut()
        && player.actor.handle == handle {
            return apply_chr_hp_delta(&mut player.actor, delta);
        }
    hud_actor_by_handle(world_chr_man, &handle)
        .map(|mut chr| apply_chr_hp_delta(&mut chr, delta))
        .or_else(|| {
            revenant_buddy_chr_by_handle_mut(world_chr_man, handle)
                .map(|chr| apply_chr_hp_delta_address(chr.address, delta))
        })
        .unwrap_or(0)
}

fn apply_chr_hp_delta(chr: &mut HudActor, delta: i32) -> i32 {
    apply_chr_hp_delta_address(chr.identity.address, delta)
}

fn apply_chr_hp_delta_address(address: u64, delta: i32) -> i32 {
    // SAFETY: This existing callback requires stable exclusive native character/vital storage.
    let result = unsafe {
        native_vitals::change(
            address,
            shikifsrs_elden_ring::character_vitals::CharacterVital::Hp,
            native_vitals::VitalChange::Delta(delta),
        )
    };
    match result {
        Ok(applied) => applied,
        Err(error) => {
            append_damage_log(&format!(
                "HP change rejected or uncertain; receipt retained: {error}"
            ));
            0
        }
    }
}

fn apply_fp_delta_by_handle(world_chr_man: &mut HudWorld, handle: HudHandle, delta: i32) -> i32 {
    if delta == 0 {
        return 0;
    }
    if let Some(player) = world_chr_man.player.as_mut()
        && player.actor.handle == handle {
            return apply_chr_fp_delta(&mut player.actor, delta);
        }
    hud_actor_by_handle(world_chr_man, &handle)
        .map(|mut chr| apply_chr_fp_delta(&mut chr, delta))
        .or_else(|| {
            revenant_buddy_chr_by_handle_mut(world_chr_man, handle)
                .map(|chr| apply_chr_fp_delta_address(chr.address, delta))
        })
        .unwrap_or(0)
}

fn apply_chr_fp_delta(chr: &mut HudActor, delta: i32) -> i32 {
    apply_chr_fp_delta_address(chr.identity.address, delta)
}

fn apply_chr_fp_delta_address(address: u64, delta: i32) -> i32 {
    // SAFETY: This existing callback requires stable exclusive native character/vital storage.
    let result = unsafe {
        native_vitals::change(
            address,
            shikifsrs_elden_ring::character_vitals::CharacterVital::Mp,
            native_vitals::VitalChange::Delta(delta),
        )
    };
    match result {
        Ok(applied) => applied,
        Err(error) => {
            append_damage_log(&format!(
                "FP change rejected or uncertain; receipt retained: {error}"
            ));
            0
        }
    }
}

fn recluse_blood_attacker_is_friendly(
    world_chr_man: &HudWorld,
    attacker: HudHandle,
    player_handle: HudHandle,
    player_team_type: u8,
) -> bool {
    if attacker.is_empty() {
        return false;
    }
    if attacker == player_handle {
        return true;
    }
    if let Some(chr) = hud_actor_by_handle(world_chr_man, &attacker) {
        return recluse_blood_chr_is_friendly(&chr, player_team_type);
    }
    revenant_buddy_chr_by_handle(world_chr_man, attacker)
        .is_some_and(|target| recluse_blood_identity_is_friendly(target, player_team_type))
}

fn recluse_blood_identity_is_friendly(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
    player_team_type: u8,
) -> bool {
    native_character::friendly_to_team(target, player_team_type, REVENANT_CONVERTED_SOUL_TEAM_TYPE)
        .map_err(|error| eprintln!("ShikiFSRS HUD friendship observation rejected: {error}"))
        .unwrap_or(false)
}

fn recluse_blood_chr_is_friendly(chr: &HudActor, player_team_type: u8) -> bool {
    match native_vitals::identity(chr.identity.address) {
        Ok(target) => recluse_blood_identity_is_friendly(target, player_team_type),
        Err(error) => {
            eprintln!("ShikiFSRS HUD friendship identity rejected: {error}");
            false
        }
    }
}

fn advance_layered_charge(layers: &mut [f32; 2], max_layers: usize, mut charge: f32) {
    for layer in layers.iter_mut().take(max_layers) {
        if charge <= 0.0 {
            break;
        }
        if *layer >= 1.0 {
            continue;
        }

        let room = 1.0 - *layer;
        let applied = charge.min(room);
        *layer += applied;
        charge -= applied;
    }
}

fn spend_skill_charge(layers: &mut [f32; 2], max_layers: usize) {
    for index in 0..max_layers {
        if layers[index] >= 1.0 {
            for shift in index..(max_layers - 1) {
                layers[shift] = layers[shift + 1];
            }
            layers[max_layers - 1] = 0.0;
            return;
        }
    }
}

fn consume_effect(player: &mut HudPlayer, active_effects: &[i32], sp_effect: i32) -> bool {
    if active_effects.contains(&sp_effect) {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, sp_effect) },
        );
        true
    } else {
        false
    }
}

fn consume_event_effect(
    player: &mut HudPlayer,
    active_effects: &[i32],
    config: &RoleConfig,
    offset: i32,
) -> bool {
    let shared = shared_effect(offset);
    let shared_active = consume_effect(player, active_effects, shared);
    let legacy_active = legacy_event_effect_id(config, offset)
        .is_some_and(|legacy| consume_effect(player, active_effects, legacy));
    shared_active || legacy_active
}

fn legacy_event_effect_id(config: &RoleConfig, offset: i32) -> Option<i32> {
    let legacy = config.effect(offset);
    // Undertaker's former ultimate aliases now belong to its buff and dodge outputs.
    let reserved = config.role == Role::Undertaker
        && matches!(
            offset,
            SP_ULTIMATE_CHARGED_USED | SP_ULTIMATE_UNCHARGED_USED
        );
    (legacy != shared_effect(offset) && !reserved).then_some(legacy)
}

fn set_effect(player: &mut HudPlayer, sp_effect: i32, active: bool) {
    if active {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::apply_effect(player.actor.identity.address, sp_effect, true)
            },
        );
    } else {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, sp_effect) },
        );
    }
}

fn remove_revenant_summon_death_effects(world_chr_man: &mut HudWorld) {
    if let Some(player) = world_chr_man.player.as_mut() {
        remove_revenant_summon_death_effects_from_player(player);
    }
}

fn remove_revenant_summon_death_effects_from_player(player: &mut HudPlayer) {
    for effect in REVENANT_SUMMON_DEATH_EFFECTS {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, effect) },
        );
    }
}

fn sync_revenant_summon_active_effects(world_chr_man: &mut HudWorld, active_index: Option<usize>) {
    let Some(player) = world_chr_man.player.as_mut() else {
        return;
    };
    for (index, effect) in REVENANT_SUMMON_ACTIVE_EFFECTS.into_iter().enumerate() {
        set_effect(player, effect, active_index == Some(index));
    }
}

fn remove_revenant_summon_active_effects(world_chr_man: &mut HudWorld) {
    if let Some(player) = world_chr_man.player.as_mut() {
        remove_revenant_summon_active_effects_from_player(player);
    }
}

fn remove_revenant_summon_active_effects_from_player(player: &mut HudPlayer) {
    for effect in REVENANT_SUMMON_ACTIVE_EFFECTS {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, effect) },
        );
    }
}

fn clear_undertaker_skill_effects(player: &mut HudPlayer) {
    set_undertaker_auto_dodge_armed(false);
    match native_vitals::identity(player.actor.identity.address) {
        Ok(target) => {
            native_character::report_effect_result(clear_undertaker_dodge_effects(target))
        }
        Err(error) => eprintln!("ShikiFSRS HUD dodge cleanup identity rejected: {error}"),
    }
    for effect in [
        SP_UNDERTAKER_NORMAL_SKILL_BUFF,
        SP_UNDERTAKER_ENHANCED_SKILL_BUFF,
    ] {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe { native_character::remove_effect(player.actor.identity.address, effect) },
        );
    }
}

fn remove_role_outputs(player: &mut HudPlayer, config: &RoleConfig) {
    for offset in [
        SP_SKILL_READY_1,
        SP_SKILL_READY_2,
        SP_ULTIMATE_READY_LEGACY_70,
        SP_ULTIMATE_READY_LEGACY_100,
        SP_ULTIMATE_READY,
    ] {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    config.effect(offset),
                )
            },
        );
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    shared_effect(offset),
                )
            },
        );
    }
    if config.role == Role::Recluse {
        clear_recluse_outputs(player, config);
    } else if config.role == Role::Scholar {
        clear_scholar_self_outputs(player, config);
    } else if config.role == Role::Revenant {
        remove_revenant_summon_death_effects_from_player(player);
        remove_revenant_summon_active_effects_from_player(player);
    } else if config.role == Role::Undertaker {
        clear_undertaker_skill_effects(player);
    }
}

fn clear_recluse_outputs(player: &mut HudPlayer, config: &RoleConfig) {
    native_character::report_effect_result(
        // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
        unsafe {
            native_character::remove_effect(
                player.actor.identity.address,
                config.effect(SP_RECLUSE_RELEASED),
            )
        },
    );
    clear_recluse_absorb_outputs(player, config);
    clear_recluse_magic_outputs(player, config);
}

fn clear_recluse_absorb_outputs(player: &mut HudPlayer, config: &RoleConfig) {
    for offset in [
        SP_RECLUSE_RESTORE_FP,
        SP_RECLUSE_ABSORB_MAGIC,
        SP_RECLUSE_ABSORB_FIRE,
        SP_RECLUSE_ABSORB_LIGHTNING,
        SP_RECLUSE_ABSORB_HOLY,
    ] {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    config.effect(offset),
                )
            },
        );
    }
}

fn clear_recluse_magic_outputs(player: &mut HudPlayer, config: &RoleConfig) {
    for index in 0..14 {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    config.effect(SP_RECLUSE_MIXED_MAGIC_BASE + index),
                )
            },
        );
    }
}

fn clear_scholar_self_outputs(player: &mut HudPlayer, config: &RoleConfig) {
    for index in 0..3 {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    player.actor.identity.address,
                    config.effect(SP_SCHOLAR_SELF_STAGE_BASE + index),
                )
            },
        );
    }
}

fn clear_scholar_enemy_outputs(enemy: &mut HudActor, config: &RoleConfig) {
    for index in 0..3 {
        native_character::report_effect_result(
            // SAFETY: The owning native callback retains the complete effect/unlink/PARAM graph.
            unsafe {
                native_character::remove_effect(
                    enemy.identity.address,
                    config.effect(SP_SCHOLAR_ENEMY_STAGE_BASE + index),
                )
            },
        );
    }
}

fn remove_all_role_outputs(player: &mut HudPlayer) {
    for config in ROLE_CONFIGS.iter() {
        remove_role_outputs(player, config);
    }
}

#[cfg(test)]
mod undertaker_skill_buff_tests {
    use super::*;

    #[test]
    fn buff_and_dodge_outputs_are_not_consumed_as_ultimate_aliases() {
        let config = role_config(Role::Undertaker);
        assert_eq!(
            config.effect(SP_ULTIMATE_CHARGED_USED),
            SP_UNDERTAKER_ENHANCED_SKILL_BUFF
        );
        assert_eq!(
            config.effect(SP_ULTIMATE_UNCHARGED_USED),
            SP_UNDERTAKER_AUTO_DODGE_TRIGGER
        );
        assert_eq!(
            legacy_event_effect_id(config, SP_ULTIMATE_CHARGED_USED),
            None
        );
        assert_eq!(
            legacy_event_effect_id(config, SP_ULTIMATE_UNCHARGED_USED),
            None
        );
        assert_eq!(shared_effect(SP_ULTIMATE_CHARGED_USED), 880021);
        assert_eq!(shared_effect(SP_ULTIMATE_UNCHARGED_USED), 880022);
        assert_eq!(legacy_event_effect_id(config, SP_SKILL_USED), Some(880911));
    }

    #[test]
    fn other_roles_retain_their_existing_ultimate_aliases() {
        for config in ROLE_CONFIGS
            .iter()
            .filter(|config| config.role != Role::Undertaker)
        {
            for offset in [SP_ULTIMATE_CHARGED_USED, SP_ULTIMATE_UNCHARGED_USED] {
                let legacy = config.effect(offset);
                let expected = (legacy != shared_effect(offset)).then_some(legacy);
                assert_eq!(legacy_event_effect_id(config, offset), expected);
            }
        }
    }

    #[test]
    fn accepted_skill_selects_the_matching_buff_and_duration() {
        let now = Instant::now();
        for (enhanced, effect, seconds) in [
            (false, SP_UNDERTAKER_NORMAL_SKILL_BUFF, 8.0),
            (true, SP_UNDERTAKER_ENHANCED_SKILL_BUFF, 15.0),
        ] {
            let mut state = ChargeState::new(now);
            state.reset_for_role(Some(Role::Undertaker));
            assert_eq!(state.begin_undertaker_skill(enhanced, now), effect);
            assert_eq!(state.undertaker_buff_duration, seconds);
            assert_eq!(
                state.undertaker_buff_end,
                Some(now + Duration::from_secs_f32(seconds))
            );
            assert_eq!(state.skill_ready_count(role_config(Role::Undertaker)), 0);
        }
    }

    #[test]
    fn buff_countdown_keeps_its_deadline_and_ends_before_cooldown() {
        let now = Instant::now();
        let config = role_config(Role::Undertaker);
        for enhanced in [false, true] {
            let mut state = ChargeState::new(now);
            state.reset_for_role(Some(Role::Undertaker));
            state.begin_undertaker_skill(enhanced, now);
            let end = state.undertaker_buff_end.unwrap();
            state.advance_skill(config, now + Duration::from_secs(1), 1.0);
            assert_eq!(state.undertaker_buff_end, Some(end));
            assert!(!state.undertaker_auto_end_pending);
            state.advance_skill(config, end, 0.1);
            assert!(state.undertaker_buff_end.is_none());
            assert!(state.undertaker_auto_end_pending);
            assert_eq!(state.skill_layers, [0.0, 0.0]);
            state.advance_skill(config, end + Duration::from_secs(7), 7.0);
            assert_eq!(state.skill_layers, [0.5, 0.0]);
        }
    }

    #[test]
    fn role_reset_clears_the_internal_buff_and_dodge_id_is_assigned() {
        let now = Instant::now();
        let mut state = ChargeState::new(now);
        state.reset_for_role(Some(Role::Undertaker));
        state.begin_undertaker_skill(true, now);
        state.reset_for_role(Some(Role::Wylder));
        assert!(state.undertaker_buff_end.is_none());
        assert!(!state.undertaker_auto_end_pending);
        assert_eq!(SP_UNDERTAKER_AUTO_DODGE_TRIGGER, 880922);
    }
}

#[cfg(test)]
mod revenant_active_effect_tests {
    use crate::HudHandle;
    use super::{
        ChargeState, REVENANT_SUMMON_COUNT, WylderHookshotNativeResolver,
        WylderHookshotNativeResult, normalized_super_armor,
        resurrection_accumulation_teammate_is_friendly,
        resurrection_accumulation_teammate_should_display, revenant_active_effect_index,
        revenant_creator_request_consumed, revenant_debug_spawn_submission_ready,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn revenant_spawn_submission_waits_for_delay_and_idle_creator() {
        let now = Instant::now();
        let ready_at = now + Duration::from_millis(100);

        assert!(!revenant_debug_spawn_submission_ready(
            false, false, now, ready_at,
        ));
        assert!(!revenant_debug_spawn_submission_ready(
            true, false, ready_at, ready_at,
        ));
        assert!(!revenant_debug_spawn_submission_ready(
            false, true, ready_at, ready_at,
        ));
        assert!(revenant_debug_spawn_submission_ready(
            false, false, ready_at, ready_at,
        ));
    }

    #[test]
    fn revenant_result_waits_for_the_submitted_creator_request_to_be_consumed() {
        assert!(!revenant_creator_request_consumed(true, true));
        assert!(revenant_creator_request_consumed(true, false));
        assert!(!revenant_creator_request_consumed(false, false));
    }

    #[test]
    fn bound_living_family_selects_its_output() {
        assert_eq!(
            revenant_active_effect_index(Some(0), None, Some(1.0)),
            Some(0)
        );
        assert_eq!(
            revenant_active_effect_index(Some(1), None, Some(0.5)),
            Some(1)
        );
        assert_eq!(
            revenant_active_effect_index(Some(2), None, Some(0.01)),
            Some(2)
        );
    }

    #[test]
    fn pending_missing_dead_or_invalid_family_has_no_output() {
        assert_eq!(
            revenant_active_effect_index(Some(0), Some(0), Some(1.0)),
            None
        );
        assert_eq!(revenant_active_effect_index(Some(0), None, None), None);
        assert_eq!(revenant_active_effect_index(Some(0), None, Some(0.0)), None);
        assert_eq!(revenant_active_effect_index(None, None, Some(1.0)), None);
        assert_eq!(revenant_active_effect_index(Some(3), None, Some(1.0)), None);
    }

    #[test]
    fn map_refresh_clears_presence_but_preserves_family_progress() {
        let mut state = ChargeState::new(Instant::now());
        state.revenant_active_summon = Some(1);
        state.revenant_effect_mask = 0b010;
        state.revenant_summon_hp = [0.25, 0.5, 0.75];
        state.revenant_summon_handles = [Some(HudHandle::none()); REVENANT_SUMMON_COUNT];
        state.revenant_summon_effect_seen = [false; REVENANT_SUMMON_COUNT];
        state.revenant_summon_death_locked = [true, false, true];
        state.revenant_summon_restore_timer = [1.0, 2.0, 3.0];
        state.revenant_buddy_count = 1;
        state.revenant_known_buddy_handles = vec![HudHandle::none()];
        state.revenant_bind_baseline = vec![HudHandle::none()];
        state.revenant_pending_summon_bind = Some(1);

        state.reset_revenant_presence_for_map_refresh();

        assert_eq!(state.revenant_active_summon, None);
        assert_eq!(state.revenant_effect_mask, 0);
        assert_eq!(state.revenant_summon_handles, [None; REVENANT_SUMMON_COUNT]);
        assert_eq!(
            state.revenant_summon_effect_seen,
            [true; REVENANT_SUMMON_COUNT]
        );
        assert_eq!(
            state.revenant_summon_restore_timer,
            [0.0; REVENANT_SUMMON_COUNT]
        );
        assert_eq!(state.revenant_buddy_count, 0);
        assert!(state.revenant_known_buddy_handles.is_empty());
        assert!(state.revenant_bind_baseline.is_empty());
        assert_eq!(state.revenant_pending_summon_bind, None);
        assert!(state.revenant_map_refresh_cleanup_pending);
        assert_eq!(state.revenant_summon_hp, [0.25, 0.5, 0.75]);
        assert_eq!(state.revenant_summon_death_locked, [true, false, true]);
    }

    #[test]
    fn super_armor_ratio_is_clamped_and_sanitized() {
        assert_eq!(normalized_super_armor(100.0, 100.0), 1.0);
        assert_eq!(normalized_super_armor(50.0, 100.0), 0.5);
        assert_eq!(normalized_super_armor(150.0, 100.0), 1.0);
        assert_eq!(normalized_super_armor(-10.0, 100.0), 0.0);
        assert_eq!(normalized_super_armor(10.0, 0.0), 0.0);
        assert_eq!(normalized_super_armor(f32::NAN, 100.0), 0.0);
        assert_eq!(normalized_super_armor(10.0, f32::INFINITY), 0.0);
    }

    #[test]
    fn resurrection_teammates_accept_coop_player_types_and_explicit_same_team() {
        for chr_type in [0, 1, 17] {
            assert!(resurrection_accumulation_teammate_is_friendly(
                chr_type, 0, 0
            ));
        }
        assert!(resurrection_accumulation_teammate_is_friendly(
            13,
            47,
            47
        ));
    }

    #[test]
    fn resurrection_teammates_reject_hostile_and_npc_player_types() {
        // Native table range is 0..23; NPC/cooperative NPC, hostile, ghosts,
        // unnamed kinds and signed out-of-range values must not pass by team alone.
        for chr_type in [i32::MIN, -1, 2, 3, 4, 5, 6, 7, 9, 14, 15, 16, 18, 19, 20, 21, 22, 23, i32::MAX] {
            assert!(!resurrection_accumulation_teammate_is_friendly(
                chr_type, 0, 0
            ));
        }
        assert!(!resurrection_accumulation_teammate_is_friendly(
            2,
            47,
            47
        ));
    }

    #[test]
    fn resurrection_teammate_display_requires_remote_visible_friendly_with_effect() {
        assert!(resurrection_accumulation_teammate_should_display(
            false, false, true, true, true
        ));
        assert!(!resurrection_accumulation_teammate_should_display(
            true, false, true, true, true
        ));
        assert!(!resurrection_accumulation_teammate_should_display(
            false, true, true, true, true
        ));
        assert!(!resurrection_accumulation_teammate_should_display(
            false, false, false, true, true
        ));
        assert!(!resurrection_accumulation_teammate_should_display(
            false, false, true, false, true
        ));
        assert!(!resurrection_accumulation_teammate_should_display(
            false, false, true, true, false
        ));
    }

    #[test]
    fn hookshot_first_enemy_result_is_latched_once() {
        let mut resolver = WylderHookshotNativeResolver::default();
        assert_eq!(resolver.update(true, None), None);
        assert_eq!(
            resolver.update(true, Some(WylderHookshotNativeResult::PullToEnemy)),
            Some(WylderHookshotNativeResult::PullToEnemy)
        );
        assert_eq!(
            resolver.update(true, Some(WylderHookshotNativeResult::PullEnemy)),
            None
        );
    }

    #[test]
    fn hookshot_inactive_edge_resets_the_previous_result() {
        let mut resolver = WylderHookshotNativeResolver::default();
        assert_eq!(
            resolver.update(true, Some(WylderHookshotNativeResult::PullEnemy)),
            Some(WylderHookshotNativeResult::PullEnemy)
        );
        assert_eq!(resolver.update(false, None), None);
        assert_eq!(
            resolver.update(true, Some(WylderHookshotNativeResult::PullToEnemy)),
            Some(WylderHookshotNativeResult::PullToEnemy)
        );
    }
}

#[cfg(test)]
mod wylder_hookshot_movement_stop_tests {
    use crate::HudHandle;
    use super::{
        HudPosition, WylderHookshotCapsule, WylderHookshotMovementDestination,
        WylderHookshotMovementStopDetector, wylder_hookshot_capsule_from_values,
        wylder_hookshot_capsules_touch,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn close_destination_waits_for_the_startup_grace_period() {
        let now = Instant::now();
        let target = HudPosition::from_xyz(10.0, 0.0, 0.0);
        let mut detector = WylderHookshotMovementStopDetector::default();
        detector.begin(
            Some(WylderHookshotMovementDestination::Point(target)),
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            now,
        );

        assert!(!detector.sample(
            HudPosition::from_xyz(9.2, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(40),
        ));
        assert!(detector.sample(
            HudPosition::from_xyz(9.2, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(100),
        ));
    }

    #[test]
    fn crossing_the_target_plane_stops_even_after_a_large_frame_step() {
        let now = Instant::now();
        let target = HudPosition::from_xyz(10.0, 0.0, 0.0);
        let mut detector = WylderHookshotMovementStopDetector::default();
        detector.begin(
            Some(WylderHookshotMovementDestination::Point(target)),
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            now,
        );

        assert!(!detector.sample(
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            Some(target),
            None,
            None,
            now,
        ));
        assert!(!detector.sample(
            HudPosition::from_xyz(8.0, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(100),
        ));
        assert!(detector.sample(
            HudPosition::from_xyz(11.5, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(120),
        ));
    }

    #[test]
    fn moving_away_after_the_closest_point_stops_the_pull() {
        let now = Instant::now();
        let target = HudPosition::from_xyz(10.0, 0.0, 0.0);
        let mut detector = WylderHookshotMovementStopDetector::default();
        detector.begin(
            Some(WylderHookshotMovementDestination::Point(target)),
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            now,
        );

        assert!(!detector.sample(
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            Some(target),
            None,
            None,
            now,
        ));
        assert!(!detector.sample(
            HudPosition::from_xyz(8.0, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(100),
        ));
        assert!(detector.sample(
            HudPosition::from_xyz(7.7, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(120),
        ));
    }

    #[test]
    fn stop_request_is_emitted_only_once_per_activation() {
        let now = Instant::now();
        let target = HudPosition::from_xyz(2.0, 0.0, 0.0);
        let mut detector = WylderHookshotMovementStopDetector::default();
        detector.begin(
            Some(WylderHookshotMovementDestination::Enemy(HudHandle::none())),
            HudPosition::from_xyz(0.0, 0.0, 0.0),
            now,
        );

        assert!(detector.sample(
            HudPosition::from_xyz(0.6, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(100),
        ));
        assert!(!detector.sample(
            HudPosition::from_xyz(0.8, 0.0, 0.0),
            Some(target),
            None,
            None,
            now + Duration::from_millis(120),
        ));
    }

    #[test]
    fn npc_chr_capsule_values_take_priority_with_hit_capsule_fallback() {
        assert_eq!(
            wylder_hookshot_capsule_from_values(1.9, 0.4, 3.0, 0.8),
            Some(WylderHookshotCapsule {
                height: 1.9,
                radius: 0.4,
            })
        );
        assert_eq!(
            wylder_hookshot_capsule_from_values(f32::NAN, 0.0, 3.0, 0.8),
            Some(WylderHookshotCapsule {
                height: 3.0,
                radius: 0.8,
            })
        );
        assert_eq!(
            wylder_hookshot_capsule_from_values(f32::NAN, 0.0, f32::INFINITY, -1.0),
            None
        );
    }

    #[test]
    fn capsule_contact_scales_with_enemy_radius_and_requires_vertical_overlap() {
        let player = WylderHookshotCapsule {
            height: 1.9,
            radius: 0.4,
        };
        let small_enemy = WylderHookshotCapsule {
            height: 1.9,
            radius: 0.4,
        };
        let large_enemy = WylderHookshotCapsule {
            height: 5.0,
            radius: 2.5,
        };
        let origin = HudPosition::from_xyz(0.0, 0.0, 0.0);

        assert!(wylder_hookshot_capsules_touch(
            origin,
            player,
            HudPosition::from_xyz(0.99, 0.0, 0.0),
            small_enemy,
        ));
        assert!(!wylder_hookshot_capsules_touch(
            origin,
            player,
            HudPosition::from_xyz(1.05, 0.0, 0.0),
            small_enemy,
        ));
        assert!(wylder_hookshot_capsules_touch(
            origin,
            player,
            HudPosition::from_xyz(3.0, 0.0, 0.0),
            large_enemy,
        ));
        assert!(!wylder_hookshot_capsules_touch(
            origin,
            player,
            HudPosition::from_xyz(0.5, 2.2, 0.0),
            small_enemy,
        ));
    }
}
