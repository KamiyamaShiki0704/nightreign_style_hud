static CAMERA_DIRECTED_LAST_POSITION: LazyLock<Mutex<Option<HavokPosition>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Copy)]
struct HudSnapshot {
    role: Role,
    skill_layers: [f32; 2],
    skill_ready: bool,
    skill_top_down: bool,
    skill_count: Option<usize>,
    ironeye_precision_aiming: bool,
    ironeye_weakness_marks: [IroneyeWeaknessSnapshot; IRONEYE_WEAKNESS_MAX_TARGETS],
    ironeye_weakness_bursts: [IroneyeWeaknessBurstSnapshot; IRONEYE_WEAKNESS_MAX_BURSTS],
    recluse_elements: [Option<RecluseElement>; 3],
    recluse_ready_magic: Option<usize>,
    recluse_lock_element: Option<RecluseElement>,
    recluse_lock_pos: Option<[f32; 2]>,
    recluse_lock_debug_points: LockDebugPoints,
    scholar_observing: bool,
    scholar_targets: [ScholarTargetSnapshot; SCHOLAR_SCAN_MAX_TARGETS],
    scholar_damage_numbers: [ScholarDamageNumberSnapshot; SCHOLAR_DAMAGE_NUMBER_MAX],
    scholar_debug: ScholarScanDebug,
    revenant_summons: [RevenantSummonSnapshot; REVENANT_SUMMON_COUNT],
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
    undertaker_free_ultimate_active: bool,
    executor_ultimate_active: bool,
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
    ironeye_weakness_marks: HashMap<FieldInsHandle, IroneyeWeaknessMark>,
    ironeye_weakness_bursts: Vec<IroneyeWeaknessBurst>,
    scholar_targets: HashMap<FieldInsHandle, ScholarTargetState>,
    scholar_observing: bool,
    scholar_debug: ScholarScanDebug,
    scholar_sympathy_hp: HashMap<FieldInsHandle, i32>,
    scholar_damage_numbers: Vec<ScholarDamageNumber>,
    scholar_native_damage_ignores: HashMap<FieldInsHandle, (i32, Instant)>,
    scholar_enemy_effect_targets: Vec<FieldInsHandle>,
    duchess_snapshots: HashMap<usize, DuchessReplaySnapshot>,
    recluse_outputs_clear_at: Option<Instant>,
    revenant_active_summon: Option<usize>,
    revenant_effect_mask: u8,
    revenant_summon_hp: [f32; REVENANT_SUMMON_COUNT],
    revenant_summon_handles: [Option<FieldInsHandle>; REVENANT_SUMMON_COUNT],
    revenant_summon_effect_seen: [bool; REVENANT_SUMMON_COUNT],
    revenant_summon_restore_timer: [f32; REVENANT_SUMMON_COUNT],
    revenant_buddy_count: usize,
    revenant_known_buddy_handles: Vec<FieldInsHandle>,
    revenant_bind_baseline: Vec<FieldInsHandle>,
    revenant_pending_summon_bind: Option<usize>,
    revenant_converted_souls: HashMap<u32, RevenantConvertedSoul>,
    revenant_debug_souls: Vec<RevenantDebugSoul>,
    revenant_seen_deaths: HashSet<FieldInsHandle>,
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
    handle: Option<FieldInsHandle>,
}

#[derive(Clone, Copy)]
struct RevenantDebugSoul {
    expires_at: Instant,
    requested_at: Instant,
    handle: Option<FieldInsHandle>,
    npc_param_id: i32,
    hide_at: Option<Instant>,
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
            recluse_outputs_clear_at: None,
            revenant_active_summon: None,
            revenant_effect_mask: 0,
            revenant_summon_hp: [1.0; REVENANT_SUMMON_COUNT],
            revenant_summon_handles: [None; REVENANT_SUMMON_COUNT],
            revenant_summon_effect_seen: [false; REVENANT_SUMMON_COUNT],
            revenant_summon_restore_timer: [0.0; REVENANT_SUMMON_COUNT],
            revenant_buddy_count: 0,
            revenant_known_buddy_handles: Vec::new(),
            revenant_bind_baseline: Vec::new(),
            revenant_pending_summon_bind: None,
            revenant_converted_souls: HashMap::new(),
            revenant_debug_souls: Vec::new(),
            revenant_seen_deaths: HashSet::new(),
        }
    }

    fn sync(&mut self, now: Instant) -> Option<HudSnapshot> {
        let delta = (now - self.last_update).as_secs_f32().clamp(0.0, 0.25);
        self.last_update = now;

        let world_chr_man = unsafe { WorldChrMan::instance_mut() }.ok()?;
        let (active_effects, locked_on_enemy) = {
            let player = world_chr_man.main_player.as_mut()?;
            update_local_damage_module(player);
            let active_effects = player
                .chr_ins
                .special_effect
                .entries()
                .map(|entry| entry.param_id)
                .collect::<Vec<_>>();
            (active_effects, player.locked_on_enemy)
        };

        let role = active_role_from_effects(&active_effects);
        set_revenant_summon_range_hook_active(role == Some(Role::Revenant));
        if role != self.role {
            if let Some(old_role) = self.role {
                let player = world_chr_man.main_player.as_mut()?;
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
            ULTIMATE_DAMAGE_EVENTS.store(0, Ordering::Relaxed);
            self.scholar_sympathy_hp.clear();
            self.scholar_damage_numbers.clear();
            self.scholar_native_damage_ignores.clear();
            self.scholar_enemy_effect_targets.clear();
            self.duchess_snapshots.clear();
            self.ironeye_weakness_marks.clear();
            self.ironeye_weakness_bursts.clear();
            let player = world_chr_man.main_player.as_mut()?;
            remove_all_role_outputs(player);
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
        self.update_scholar_sympathy(world_chr_man, role, &active_effects, locked_on_enemy, now);
        self.update_ironeye_weakness(world_chr_man, role, now);
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
        let world_chr_man_ptr = world_chr_man as *mut WorldChrMan;
        let player = world_chr_man.main_player.as_mut()?;
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
            world_chr_man_ptr,
            now,
        );
        self.sync_output_speffects(player, config);

        Some(self.snapshot(world_chr_man, config, now))
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
        self.revenant_summon_restore_timer = [0.0; REVENANT_SUMMON_COUNT];
        self.revenant_buddy_count = 0;
        self.revenant_known_buddy_handles.clear();
        self.revenant_bind_baseline.clear();
        self.revenant_pending_summon_bind = None;
        self.revenant_converted_souls.clear();
        self.revenant_debug_souls.clear();
        self.revenant_seen_deaths.clear();
    }

    fn update_revenant_summons(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        role: Role,
        active_effects: &[i32],
        delta: f32,
    ) {
        if role != Role::Revenant {
            self.revenant_active_summon = None;
            self.revenant_effect_mask = 0;
            self.revenant_summon_handles = [None; REVENANT_SUMMON_COUNT];
            self.revenant_summon_effect_seen = [false; REVENANT_SUMMON_COUNT];
            self.revenant_summon_restore_timer = [0.0; REVENANT_SUMMON_COUNT];
            self.revenant_buddy_count = 0;
            self.revenant_known_buddy_handles.clear();
            self.revenant_bind_baseline.clear();
            self.revenant_pending_summon_bind = None;
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
                    self.revenant_summon_restore_timer[index] =
                        REVENANT_SUMMON_HP_RESTORE_SECONDS;
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
                revenant_buddy_group_by_param_id_mut(world_chr_man, REVENANT_BUDDY_PARAM_IDS[index])
            {
                if group_chr.disappear_requested && !should_restore_hp {
                    removed_by_system = true;
                } else {
                    if should_restore_hp {
                        apply_revenant_buddy_hp_ratio(group_chr.chr, restore_hp_ratio);
                    }
                    hp_ratio = if should_restore_hp {
                        Some(restore_hp_ratio)
                    } else {
                        revenant_buddy_hp_ratio(group_chr.chr)
                    };
                    if hp_ratio.is_some() {
                        self.revenant_summon_handles[index] = Some(group_chr.chr.field_ins_handle);
                        self.revenant_pending_summon_bind = None;
                        self.revenant_bind_baseline.clear();
                    }
                }
            }

            if !removed_by_system && hp_ratio.is_none() {
                hp_ratio = self.revenant_summon_handles[index].and_then(|handle| {
                    if should_restore_hp {
                        if let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                            apply_revenant_buddy_hp_ratio(chr, restore_hp_ratio);
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
                    if bind_was_pending {
                        if let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                            apply_revenant_buddy_hp_ratio(chr, restore_hp_ratio);
                        }
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
        if self.revenant_pending_summon_bind.is_none() {
            self.revenant_known_buddy_handles = current_buddy_handles;
        }
    }

    fn update_revenant_passive(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        role: Role,
        now: Instant,
    ) {
        if role != Role::Revenant {
            return;
        }

        if !REVENANT_PASSIVE_BUDDY_ROUTE_ENABLED {
            for soul in self.revenant_converted_souls.values() {
                release_revenant_passive_soul(world_chr_man, soul);
            }
            self.revenant_converted_souls.clear();
            self.update_revenant_debug_souls(world_chr_man, now);
            self.try_spawn_revenant_debug_kills(world_chr_man, now);
            return;
        }

        self.update_revenant_converted_souls(world_chr_man, now);
        self.try_convert_revenant_kills(world_chr_man, now);
    }

    fn update_revenant_converted_souls(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        now: Instant,
    ) {
        for soul in self.revenant_converted_souls.values_mut() {
            if soul.handle.is_none() {
                if let Some(group_chr) =
                    revenant_buddy_group_by_param_id_mut(world_chr_man, soul.buddy_param_id)
                {
                    prepare_revenant_passive_buddy(group_chr.chr);
                    soul.handle = Some(group_chr.chr.field_ins_handle);
                }
            } else if let Some(handle) = soul.handle {
                if let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
                    prepare_revenant_passive_buddy(chr);
                }
            }
        }

        let mut expired = Vec::new();
        for (buddy_param_id, soul) in &self.revenant_converted_souls {
            let should_remove = now >= soul.expires_at
                || soul.handle.map_or(false, |handle| {
                    revenant_buddy_chr_by_handle(world_chr_man, handle)
                        .map_or(true, revenant_converted_soul_should_disappear)
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

    fn try_convert_revenant_kills(&mut self, world_chr_man: &mut WorldChrMan, now: Instant) {
        let Some(player_handle) = world_chr_man
            .main_player
            .as_ref()
            .map(|player| player.chr_ins.field_ins_handle)
        else {
            return;
        };

        let mut candidates = Vec::new();
        for entry in world_chr_man.chr_inses_by_distance.iter() {
            let chr = unsafe { entry.chr_ins.as_ref() };
            let handle = chr.field_ins_handle;
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

    fn clear_revenant_converted_souls(&mut self, world_chr_man: &mut WorldChrMan) {
        for soul in self.revenant_converted_souls.values() {
            release_revenant_passive_soul(world_chr_man, soul);
        }
        self.revenant_converted_souls.clear();
        for soul in self.revenant_debug_souls.drain(..) {
            release_revenant_debug_soul(world_chr_man, &soul);
        }
        self.revenant_seen_deaths.clear();
    }

    fn update_revenant_debug_souls(&mut self, world_chr_man: &mut WorldChrMan, now: Instant) {
        for soul in &mut self.revenant_debug_souls {
            if soul.hide_at.is_some() {
                continue;
            }
            if soul.handle.is_none() {
                if let Some(mut chr_ptr) = world_chr_man.debug_chr_creator.last_created_chr {
                    let chr = unsafe { chr_ptr.as_mut() };
                    if chr.npc_param_id == soul.npc_param_id {
                        prepare_revenant_debug_soul(chr);
                        request_revenant_debug_soul_animation(
                            chr,
                            REVENANT_CONVERTED_SOUL_APPEAR_ANIM_ID,
                        );
                        soul.handle = Some(chr.field_ins_handle);
                    }
                }
            } else if let Some(handle) = soul.handle {
                if let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) {
                    prepare_revenant_debug_soul(chr);
                }
            }
        }

        let mut index = 0;
        while index < self.revenant_debug_souls.len() {
            let soul = self.revenant_debug_souls[index];
            if let Some(hide_at) = soul.hide_at {
                let died_while_hiding = soul.handle.map_or(false, |handle| {
                    revenant_debug_chr_by_handle(world_chr_man, handle)
                        .map_or(true, revenant_converted_soul_should_disappear)
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
            let died_or_missing = soul.handle.map_or(false, |handle| {
                    revenant_debug_chr_by_handle(world_chr_man, handle)
                        .map_or(true, revenant_converted_soul_should_disappear)
                });
            if died_or_missing {
                let soul = self.revenant_debug_souls.swap_remove(index);
                release_revenant_debug_soul(world_chr_man, &soul);
            } else if expired_by_time {
                if let Some(handle) = soul.handle {
                    if let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) {
                        request_revenant_debug_soul_animation(
                            chr,
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

    fn try_spawn_revenant_debug_kills(&mut self, world_chr_man: &mut WorldChrMan, now: Instant) {
        let Some(player_handle) = world_chr_man
            .main_player
            .as_ref()
            .map(|player| player.chr_ins.field_ins_handle)
        else {
            return;
        };

        if self.revenant_debug_souls.len() >= REVENANT_CONVERTED_SOUL_LIMIT {
            return;
        }

        let mut candidates = Vec::new();
        for entry in world_chr_man.chr_inses_by_distance.iter() {
            let chr = unsafe { entry.chr_ins.as_ref() };
            let handle = chr.field_ins_handle;
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

            world_chr_man.debug_chr_creator.last_created_chr = None;
            world_chr_man.spawn_debug_character(&ChrDebugSpawnRequest {
                chr_id: candidate.npc_id,
                chara_init_param_id: 0,
                npc_param_id: candidate.npc_param_id,
                npc_think_param_id: candidate.npc_think_param_id,
                event_entity_id: 0,
                talk_id: 0,
                pos_x: candidate.position.0,
                pos_y: candidate.position.1,
                pos_z: candidate.position.2,
            });
            self.revenant_debug_souls.push(RevenantDebugSoul {
                expires_at: now + Duration::from_secs_f32(REVENANT_CONVERTED_SOUL_SECONDS),
                requested_at: now,
                handle: None,
                npc_param_id: candidate.npc_param_id,
                hide_at: None,
            });
            break;
        }
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
        if config.role == Role::Undertaker {
            if self
                .undertaker_free_ultimate_end
                .is_some_and(|end| (end - now).as_secs_f32() <= 0.0)
            {
                self.undertaker_free_ultimate_end = None;
            }
        }

        if config.role == Role::Executor {
            if let Some(end) = self.executor_transform_end {
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
        }

        self.ultimate = (self.ultimate + delta / config.ultimate_cooldown).clamp(0.0, 1.0);
    }

    fn apply_pending_undertaker_events(
        &mut self,
        player: &mut eldenring::cs::PlayerIns,
        config: &RoleConfig,
    ) {
        if config.role == Role::Undertaker && self.undertaker_auto_end_pending {
            player.apply_speffect(config.effect(SP_UNDERTAKER_SKILL_AUTO_END), true);
            self.undertaker_auto_end_pending = false;
        }
    }

    fn apply_pending_executor_events(
        &mut self,
        player: &mut eldenring::cs::PlayerIns,
        config: &RoleConfig,
    ) {
        if config.role == Role::Executor && self.executor_auto_end_pending {
            player.apply_speffect(config.effect(SP_EXECUTOR_ULTIMATE_AUTO_END), true);
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
        world_chr_man: &WorldChrMan,
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
            let Some(player) = world_chr_man.main_player.as_ref() else {
                self.decay_scholar_targets(world_chr_man, delta);
                return;
            };
            let player_pos = player.chr_ins.modules.as_ref().physics.as_ref().position;

            let mut debug_candidate_score = f32::MAX;
            let mut scanned = 0usize;
            for entry in world_chr_man.chr_inses_by_distance.iter() {
                if scanned >= SCHOLAR_SCAN_MAX_DISTANCE_ENTRIES {
                    break;
                }
                scanned += 1;
                self.scholar_debug.scanned += 1;

                let chr = unsafe { entry.chr_ins.as_ref() };
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
                if !scholar_in_lock_target_volume(player.as_ref(), target_pos, target_distance) {
                    self.scholar_debug.range_skip += 1;
                    continue;
                }
                let scan_gain = scholar_scan_gain(target_distance);
                if scan_gain <= 0.0 {
                    self.scholar_debug.range_skip += 1;
                    continue;
                }
                if let Some(score) = scholar_screen_pos_lens_score(screen_pos) {
                    if score < debug_candidate_score {
                        debug_candidate_score = score;
                        self.scholar_debug.candidate =
                            scholar_candidate_debug(chr, target_distance, player.as_ref());
                    }
                }
                if !scholar_has_line_of_sight(chr, player.as_ref()) {
                    self.scholar_debug.los_skip += 1;
                    continue;
                }
                let gain = scan_gain * delta;
                let target = self
                    .scholar_targets
                    .entry(chr.field_ins_handle)
                    .or_default();
                target.progress = (target.progress + gain).clamp(0.0, 1.0);
                target.pos = screen_pos;
                target.visible = true;
                target.handle = Some(chr.field_ins_handle);
                self.scholar_debug.accepted += 1;
            }
        }

        self.decay_scholar_targets(world_chr_man, delta);
    }

    fn decay_scholar_targets(&mut self, world_chr_man: &WorldChrMan, delta: f32) {
        let mut expired = Vec::new();
        for (handle, target) in self.scholar_targets.iter_mut() {
            if !target.visible {
                let Some(chr) = world_chr_man.chr_ins_by_handle(handle) else {
                    expired.push(*handle);
                    continue;
                };
                if scholar_reject_reason(chr).is_some() {
                    expired.push(*handle);
                    continue;
                }
                let Some(screen_pos) = scholar_target_screen_pos(chr) else {
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

    fn update_ironeye_weakness(&mut self, world_chr_man: &mut WorldChrMan, role: Role, now: Instant) {
        let config = role_config(Role::Ironeye);
        if role != Role::Ironeye {
            self.clear_ironeye_weakness_marks(world_chr_man, config);
            self.ironeye_weakness_bursts.clear();
            drain_damage_hook_events();
            return;
        }

        self.ironeye_weakness_bursts
            .retain(|burst| (now - burst.created_at).as_secs_f32() < IRONEYE_WEAKNESS_BURST_SECONDS);
        self.expire_ironeye_weakness_marks(world_chr_man, config, now);
        self.refresh_ironeye_weakness_marks(world_chr_man, config, now);
        self.accumulate_ironeye_weakness_damage(world_chr_man, config, now);
    }

    fn refresh_ironeye_weakness_marks(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
        now: Instant,
    ) {
        let trigger = config.effect(SP_IRONEYE_WEAKNESS_TRIGGER);
        let mut handles = Vec::new();
        for entry in world_chr_man.chr_inses_by_distance.iter() {
            let chr = unsafe { entry.chr_ins.as_ref() };
            if scholar_reject_reason(chr).is_some() || !chr_has_speffect(chr, trigger) {
                continue;
            }
            handles.push(chr.field_ins_handle);
        }

        for handle in handles {
            let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) else {
                continue;
            };
            enemy.remove_speffect(trigger);
            if scholar_reject_reason(enemy).is_some() {
                continue;
            }
            let data = enemy.modules.as_ref().data.as_ref();
            let damage_module = damage_module_from_chr_ins(enemy);
            if damage_module == 0 || data.max_hp <= 0 {
                continue;
            }
            let threshold_damage =
                ((data.max_hp as f32) * IRONEYE_WEAKNESS_THRESHOLD_MAX_HP_RATE).ceil() as i32;
            enemy.apply_speffect(config.effect(SP_IRONEYE_WEAKNESS_ACTIVE), true);
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
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
        now: Instant,
    ) {
        let events = drain_damage_hook_events();
        if events.is_empty() || self.ironeye_weakness_marks.is_empty() {
            return;
        }

        let mut broken = Vec::new();
        for event in events {
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

        broken.sort_by_key(|handle| (handle.selector.container(), handle.selector.index()));
        broken.dedup();
        for handle in broken {
            self.break_ironeye_weakness_mark(world_chr_man, config, handle);
        }
    }

    fn break_ironeye_weakness_mark(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
        handle: FieldInsHandle,
    ) {
        if self.ironeye_weakness_marks.remove(&handle).is_none() {
            return;
        }
        let burst_pos = world_chr_man
            .chr_ins_by_handle(&handle)
            .and_then(scholar_target_screen_pos);
        if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
            enemy.remove_speffect(config.effect(SP_IRONEYE_WEAKNESS_ACTIVE));
            enemy.apply_speffect(config.effect(SP_IRONEYE_WEAKNESS_BREAK), true);
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
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
        now: Instant,
    ) {
        let mut expired = Vec::new();
        for (&handle, mark) in self.ironeye_weakness_marks.iter() {
            let invalid = world_chr_man
                .chr_ins_by_handle(&handle)
                .map_or(true, |enemy| scholar_reject_reason(enemy).is_some());
            if now >= mark.expires_at || invalid {
                expired.push(handle);
            }
        }
        for handle in expired {
            self.ironeye_weakness_marks.remove(&handle);
            if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
                enemy.remove_speffect(config.effect(SP_IRONEYE_WEAKNESS_ACTIVE));
            }
        }
    }

    fn clear_ironeye_weakness_marks(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
    ) {
        for handle in self.ironeye_weakness_marks.drain().map(|(handle, _)| handle) {
            if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
                enemy.remove_speffect(config.effect(SP_IRONEYE_WEAKNESS_ACTIVE));
            }
        }
    }

    fn update_duchess_replay_history(&mut self, world_chr_man: &WorldChrMan, role: Role) {
        if role != Role::Duchess {
            self.duchess_snapshots.clear();
            return;
        }

        let mut seen = Vec::new();
        for entry in world_chr_man.chr_inses_by_distance.iter() {
            let enemy = unsafe { entry.chr_ins.as_ref() };
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
        world_chr_man: &mut WorldChrMan,
        role: Role,
        active_effects: &[i32],
        locked_on_enemy: FieldInsHandle,
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
        self.scholar_damage_numbers
            .retain(|number| (now - number.created_at).as_secs_f32() < SCHOLAR_DAMAGE_NUMBER_SECONDS);
    }

    fn prune_scholar_native_damage_ignores(&mut self, now: Instant) {
        self.scholar_native_damage_ignores
            .retain(|_, (_, expires_at)| *expires_at > now);
    }

    fn apply_scholar_sympathy_hp_losses(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        links: &[ScholarLink],
        locked_on_enemy: FieldInsHandle,
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
        world_chr_man: &mut WorldChrMan,
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
            for ally in links.iter().filter(|link| scholar_link_is_friendly(*link)) {
                apply_hp_delta_by_handle(world_chr_man, ally.handle, ally_heal);
            }
        }
    }

    fn apply_scholar_sympathy_self_counter(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        links: &[ScholarLink],
        source: &ScholarLink,
        damage: i32,
        locked_on_enemy: FieldInsHandle,
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
        world_chr_man: &mut WorldChrMan,
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
        world_chr_man: &mut WorldChrMan,
        links: &[ScholarLink],
        now: Instant,
    ) {
        for source in links.iter().filter(|link| scholar_link_is_friendly(*link)) {
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
                    .filter(|link| scholar_link_is_friendly(*link) && link.handle != source.handle)
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
        world_chr_man: &mut WorldChrMan,
        handle: FieldInsHandle,
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
        world_chr_man: &mut WorldChrMan,
        handle: FieldInsHandle,
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
        world_chr_man: &WorldChrMan,
        handle: FieldInsHandle,
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

    fn add_scholar_speffect_damage_ignore(&mut self, handle: FieldInsHandle, now: Instant) {
        let expires_at = now + Duration::from_secs_f32(SCHOLAR_SPEFFECT_DAMAGE_IGNORE_SECONDS);
        self.scholar_native_damage_ignores
            .entry(handle)
            .and_modify(|(amount, existing_expires_at)| {
                *amount = i32::MAX / 4;
                *existing_expires_at = expires_at;
            })
            .or_insert((i32::MAX / 4, expires_at));
    }

    fn consume_scholar_native_damage_ignore(
        &mut self,
        handle: FieldInsHandle,
        damage: i32,
    ) -> i32 {
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

    fn refresh_scholar_sympathy_hp(&mut self, world_chr_man: &WorldChrMan, links: &[ScholarLink]) {
        let mut next = HashMap::new();
        for link in links {
            if let Some(hp) = hp_by_handle(world_chr_man, link.handle) {
                next.insert(link.handle, hp);
            }
        }
        self.scholar_sympathy_hp = next;
    }

    fn consume_inputs(
        &mut self,
        player: &mut eldenring::cs::PlayerIns,
        config: &RoleConfig,
        active_effects: &[i32],
        locked_element: Option<RecluseElement>,
        locked_damage_module: Option<usize>,
        world_chr_man: *mut WorldChrMan,
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
                    }
                } else if skill_enhanced_use_allowed && self.skill_layers[0] >= 1.0 {
                    self.undertaker_buff_duration = UNDERTAKER_ENHANCED_BUFF_SECONDS;
                    self.undertaker_buff_end =
                        Some(now + Duration::from_secs_f32(UNDERTAKER_ENHANCED_BUFF_SECONDS));
                    self.skill_layers = [1.0, 0.0];
                } else if skill_use_allowed && self.skill_layers[0] >= 1.0 {
                    self.undertaker_buff_duration = UNDERTAKER_BUFF_SECONDS;
                    self.undertaker_buff_end =
                        Some(now + Duration::from_secs_f32(UNDERTAKER_BUFF_SECONDS));
                    self.skill_layers = [1.0, 0.0];
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

    fn apply_scholar_self_effect(
        &mut self,
        player: &mut eldenring::cs::PlayerIns,
        config: &RoleConfig,
    ) {
        if let Some(stage) = self.best_scholar_stage() {
            clear_scholar_self_outputs(player, config);
            player.apply_speffect(
                config.effect(SP_SCHOLAR_SELF_STAGE_BASE + stage as i32 - 1),
                true,
            );
        }
    }

    fn apply_scholar_enemy_effect(&mut self, world_chr_man: *mut WorldChrMan, config: &RoleConfig) {
        let Some(world_chr_man) = (unsafe { world_chr_man.as_mut() }) else {
            return;
        };
        let targets = self.scholar_enemy_effect_targets();
        self.clear_scholar_enemy_effects(world_chr_man, config);
        for (handle, stage) in targets {
            let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) else {
                continue;
            };
            if scholar_reject_reason(enemy).is_some() {
                continue;
            }
            clear_scholar_enemy_outputs(enemy, config);
            enemy.apply_speffect(
                config.effect(SP_SCHOLAR_ENEMY_STAGE_BASE + stage as i32 - 1),
                true,
            );
            self.scholar_enemy_effect_targets.push(handle);
        }
    }

    fn scholar_enemy_effect_targets(&self) -> Vec<(FieldInsHandle, usize)> {
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

    fn clear_scholar_enemy_effects(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        config: &RoleConfig,
    ) {
        for handle in self.scholar_enemy_effect_targets.drain(..) {
            if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
                clear_scholar_enemy_outputs(enemy, config);
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

    fn best_scholar_target(&self) -> Option<(FieldInsHandle, ScholarTargetState)> {
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
        player: &mut eldenring::cs::PlayerIns,
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
        player.apply_speffect(config.effect(SP_RECLUSE_RESTORE_FP), true);
        player.apply_speffect(config.effect(element.absorb_effect_offset()), true);
        if let Some(magic_index) = self.recluse_ready_magic {
            clear_recluse_magic_outputs(player, config);
            player.apply_speffect(
                config.effect(SP_RECLUSE_MIXED_MAGIC_BASE + magic_index as i32 - 1),
                true,
            );
        }
        self.recluse_outputs_clear_at =
            Some(now + Duration::from_secs_f32(RECLUSE_TRANSIENT_OUTPUT_SECONDS));
    }

    fn handle_duchess_skill(
        &mut self,
        player: &mut eldenring::cs::PlayerIns,
        world_chr_man: *mut WorldChrMan,
        now: Instant,
    ) {
        let Some(world_chr_man) = (unsafe { world_chr_man.as_mut() }) else {
            return;
        };
        let player_pos = player.chr_ins.modules.as_ref().physics.as_ref().position;
        let mut targets = Vec::new();

        for entry in world_chr_man.chr_inses_by_distance.iter() {
            let enemy = unsafe { entry.chr_ins.as_ref() };
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
                targets.push((enemy.field_ins_handle, damage_module, payload));
            }
        }

        for (handle, damage_module, payload) in targets {
            self.apply_duchess_replay(world_chr_man, handle, damage_module, payload);
        }
    }

    fn apply_duchess_replay(
        &mut self,
        world_chr_man: &mut WorldChrMan,
        handle: FieldInsHandle,
        damage_module: usize,
        payload: DuchessReplayPayload,
    ) {
        if !payload.has_effect() {
            return;
        }
        if set_duchess_replay_speffect(payload) {
            if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
                add_duchess_replay_ignore(damage_module, payload);
                enemy.apply_speffect(SP_DUCHESS_REPLAY_DAMAGE, true);
                apply_duchess_replay_posture_damage(enemy, payload);
            }
            return;
        }

        if let Some(enemy) = world_chr_man.chr_ins_by_handle_mut(&handle) {
            if payload.hp_damage > 0 {
                apply_chr_hp_delta(enemy, -payload.hp_damage);
            }
            add_duchess_replay_ignore(
                damage_module,
                DuchessReplayPayload {
                    hp_damage: payload.hp_damage,
                    status: [0; 7],
                    super_armor_damage: payload.super_armor_damage,
                },
            );
            apply_duchess_replay_posture_damage(enemy, payload);
        }
    }

    fn clear_recluse_magic(&mut self, player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
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
        player: &mut eldenring::cs::PlayerIns,
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

    fn sync_output_speffects(&self, player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
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
        world_chr_man: &WorldChrMan,
        config: &RoleConfig,
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
        if config.skill_kind == SkillKind::UndertakerBuff {
            if let Some(end) = self.undertaker_buff_end {
                let remaining = (end - now).as_secs_f32();
                skill_layers = [
                    (remaining / self.undertaker_buff_duration).clamp(0.0, 1.0),
                    0.0,
                ];
            }
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
            for (slot, (handle, mark)) in ironeye_weakness_marks
                .iter_mut()
                .zip(self.ironeye_weakness_marks.iter())
            {
                let Some(pos) = world_chr_man
                    .chr_ins_by_handle(handle)
                    .and_then(scholar_target_screen_pos)
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
            ironeye_precision_aiming: config.role == Role::Ironeye
                && IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed),
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
            scholar_observing: config.role == Role::Scholar && self.scholar_observing,
            scholar_targets,
            scholar_damage_numbers,
            scholar_debug: if config.role == Role::Scholar {
                self.scholar_debug
            } else {
                ScholarScanDebug::default()
            },
            revenant_summons,
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
                    .map(|handle| handle.selector.container())
                    .unwrap_or(u32::MAX)
            } else {
                u32::MAX
            },
            revenant_active_handle_index: if config.role == Role::Revenant {
                self.revenant_active_summon
                    .and_then(|index| self.revenant_summon_handles[index])
                    .map(|handle| handle.selector.index())
                    .unwrap_or(u32::MAX)
            } else {
                u32::MAX
            },
            revenant_active_npc_id: if config.role == Role::Revenant {
                self.revenant_active_summon
                    .and_then(|index| self.revenant_summon_handles[index])
                    .and_then(|handle| world_chr_man.chr_ins_by_handle(&handle))
                    .map(|chr| chr.npc_id)
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

    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    let current_position = player.chr_ins.modules.as_ref().physics.as_ref().position;
    if !chr_has_speffect(&player.chr_ins, CAMERA_DIRECTED_MOVEMENT_EFFECT) {
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
        || horizontal_len < CAMERA_DIRECTED_MIN_FRAME_DELTA
        || horizontal_len > CAMERA_DIRECTED_MAX_FRAME_DELTA
        || dy.abs() > CAMERA_DIRECTED_MAX_FRAME_DELTA
    {
        *last_position = Some(current_position);
        return;
    }

    let vertical_delta =
        (camera_pitch * horizontal_len).clamp(-CAMERA_DIRECTED_MAX_VERTICAL_DELTA, CAMERA_DIRECTED_MAX_VERTICAL_DELTA);
    let redirected_position = HavokPosition(
        current_position.0,
        previous_position.1 + vertical_delta,
        current_position.2,
        current_position.3,
    );

    {
        let physics = player.chr_ins.modules.as_mut().physics.as_mut();
        physics.position = redirected_position;
        physics.chr_proxy_pos_update_requested = true;
        physics.standing_on_solid_ground = true;
        physics.touching_solid_ground = true;
    }
    player
        .chr_ins
        .chr_ctrl
        .as_mut()
        .chr_proxy_flags
        .set_position_sync_requested(true);

    *last_position = Some(redirected_position);
}

fn camera_pitch_direction() -> Option<f32> {
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
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
const REVENANT_PASSIVE_BUDDY_ROUTE_ENABLED: bool = false;
const REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID: i32 = -1;
const REVENANT_PASSIVE_BUDDY_PARAM_IDS: [u32; REVENANT_CONVERTED_SOUL_LIMIT] = [
    52950000, 52950001, 52950002, 52950003, 52950004, 52950005, 52950006, 52950007, 52950008,
    52950009,
];

#[derive(Clone, Copy)]
struct RevenantPassiveCandidate {
    handle: FieldInsHandle,
    npc_id: i32,
    npc_param_id: i32,
    npc_think_param_id: i32,
    position: HavokPosition,
}

fn revenant_passive_candidate(
    chr: &ChrIns,
    player_handle: FieldInsHandle,
) -> Option<RevenantPassiveCandidate> {
    if chr.field_ins_handle.is_empty()
        || chr.field_ins_handle == player_handle
        || chr.last_hit_by != player_handle
        || chr.team_type == REVENANT_CONVERTED_SOUL_TEAM_TYPE
        || chr.chr_type != ChrType::Npc
        || REVENANT_BUDDY_PARAM_IDS.contains(&(chr.npc_param_id as u32))
    {
        return None;
    }

    let data = chr.modules.as_ref().data.as_ref();
    if data.hp > 0 && !chr.chr_flags1c5.death_flag() {
        return None;
    }

    let npc_think_param_id = revenant_enemy_think_param(chr)?;
    if chr.npc_id <= 0
        || chr.npc_id == 1000
        || chr.npc_param_id <= 0
        || npc_think_param_id <= 0
    {
        return None;
    }

    Some(RevenantPassiveCandidate {
        handle: chr.field_ins_handle,
        npc_id: chr.npc_id,
        npc_param_id: chr.npc_param_id,
        npc_think_param_id,
        position: chr.modules.as_ref().physics.as_ref().position,
    })
}

fn revenant_enemy_think_param(chr: &ChrIns) -> Option<i32> {
    chr.as_subclass::<EnemyIns>()
        .map(|enemy| enemy.npc_think_param)
        .filter(|think_param| *think_param > 0)
}

fn revenant_converted_soul_should_disappear(chr: &ChrIns) -> bool {
    let data = chr.modules.as_ref().data.as_ref();
    data.hp <= 0 || chr.chr_flags1c5.death_flag() || chr.debug_flags.character_disabled()
}

fn revenant_passive_roll(handle: FieldInsHandle, now: Instant) -> bool {
    let mut value = handle.selector.0 as u64
        ^ ((handle.block_id.area() as u64) << 32)
        ^ ((handle.block_id.block() as u64) << 40)
        ^ (now.elapsed().as_nanos() as u64);
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
    let Ok(param_repository) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return None;
    };

    for buddy_param_id in REVENANT_PASSIVE_BUDDY_PARAM_IDS {
        if active_souls.contains_key(&buddy_param_id) {
            continue;
        }

        let Some(row) = param_repository.get_mut::<BuddyParam>(buddy_param_id) else {
            continue;
        };
        let trigger_effect = row.trigger_sp_effect_id();
        if trigger_effect <= 0 {
            continue;
        }

        row.set_npc_param_id(npc_param_id);
        row.set_npc_think_param_id(npc_think_param_id);
        row.set_npc_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
        row.set_npc_think_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
        return Some((buddy_param_id, trigger_effect));
    }

    None
}

fn request_revenant_passive_summon(world_chr_man: &mut WorldChrMan, trigger_effect: i32) {
    world_chr_man
        .summon_buddy_manager
        .item_use_cooldown_timer = 0.0;
    world_chr_man
        .summon_buddy_manager
        .request_summon_speffect_id = trigger_effect;

    if let Some(player) = world_chr_man.main_player.as_deref_mut() {
        player.apply_speffect(trigger_effect, true);
    }
}

fn prepare_revenant_passive_buddy(chr: &mut ChrIns) {
    if !chr_has_speffect(chr, REVENANT_CONVERTED_SOUL_SPEFFECT) {
        chr.apply_speffect(REVENANT_CONVERTED_SOUL_SPEFFECT, true);
    }

    chr.team_type = REVENANT_CONVERTED_SOUL_TEAM_TYPE;
    chr.last_hit_by = FieldInsHandle::none();
    chr.load_state.set_extinction_death(false);
    chr.chr_flags1c4.set_is_render_group_enabled(true);
    chr.chr_flags1c5.set_death_flag(false);
    chr.chr_flags1c5.set_enable_render(true);
    chr.chr_flags1c8.set_is_active(true);
    chr.chr_activation_flags.set_activation_enabled(true);
    chr.debug_flags.set_force_unloaded(false);
    chr.debug_flags.set_force_loaded(true);
    chr.debug_flags.set_character_disabled(false);
    chr.debug_flags.set_disabled_updates(false);
    chr.debug_flags.set_disabled_hit(false);
    chr.debug_flags.set_disabled_movement(false);
    chr.debug_flags.set_disabled_secondary_actions(false);
    chr.opacity_keyframes_multiplier = 1.0;
    chr.opacity_keyframes_multiplier_previous = 1.0;
    chr.tint_alpha_multiplier = 1.0;
    chr.tint_alpha_multiplier_modifier = 1.0;
    chr.camouflage_transparency = 1.0;
    chr.base_transparency = 1.0;
    chr.base_transparency_modifier = 1.0;

    let data = chr.modules.as_mut().data.as_mut();
    let max_hp = data.max_hp.max(1);
    if data.hp <= 0 {
        data.hp = max_hp;
    }
}

fn prepare_revenant_debug_soul(chr: &mut ChrIns) {
    prepare_revenant_passive_buddy(chr);
    chr.modules
        .action_flag
        .action_modifiers_flags
        .set_disable_floating_gauge_display(false);
}

fn request_revenant_debug_soul_animation(chr: &mut ChrIns, anim_id: i32) {
    chr.modules.as_mut().event.as_mut().request_animation_id = anim_id;
}

fn release_revenant_debug_soul(world_chr_man: &mut WorldChrMan, soul: &RevenantDebugSoul) {
    let Some(handle) = soul.handle else {
        return;
    };
    let Some(chr) = revenant_debug_chr_by_handle_mut(world_chr_man, handle) else {
        return;
    };

    let data = chr.modules.as_mut().data.as_mut();
    data.hp = 0;
    chr.chr_flags1c5.set_death_flag(true);
    chr.chr_flags1c5.set_enable_render(false);
    chr.chr_flags1c8.set_is_active(false);
    chr.debug_flags.set_force_loaded(false);
    chr.debug_flags.set_force_unloaded(true);
    chr.debug_flags.set_character_disabled(true);
    chr.debug_flags.set_disabled_updates(true);
    chr.debug_flags.set_disabled_hit(true);
    chr.debug_flags.set_disabled_movement(true);
    chr.debug_flags.set_disabled_secondary_actions(true);
    chr.base_transparency = 0.0;
    chr.base_transparency_modifier = 0.0;
    chr.tint_alpha_multiplier = 0.0;
    chr.tint_alpha_multiplier_modifier = 0.0;
}

fn release_revenant_passive_soul(world_chr_man: &mut WorldChrMan, soul: &RevenantConvertedSoul) {
    clear_revenant_passive_buddy_slot(soul.buddy_param_id);

    if let Some(group) = revenant_buddy_group_by_param_id_mut(world_chr_man, soul.buddy_param_id) {
        let data = group.chr.modules.as_mut().data.as_mut();
        data.hp = 0;
        group.chr.chr_flags1c5.set_death_flag(true);
        group.chr.chr_flags1c5.set_enable_render(false);
        group.chr.chr_flags1c8.set_is_active(false);
        group.chr.debug_flags.set_force_loaded(false);
        group.chr.debug_flags.set_force_unloaded(true);
        group.chr.debug_flags.set_character_disabled(true);
        group.chr.debug_flags.set_disabled_updates(true);
    } else if let Some(handle) = soul.handle {
        if let Some(chr) = revenant_buddy_chr_by_handle_mut(world_chr_man, handle) {
            let data = chr.modules.as_mut().data.as_mut();
            data.hp = 0;
            chr.chr_flags1c5.set_death_flag(true);
            chr.chr_flags1c5.set_enable_render(false);
            chr.chr_flags1c8.set_is_active(false);
            chr.debug_flags.set_force_loaded(false);
            chr.debug_flags.set_force_unloaded(true);
            chr.debug_flags.set_character_disabled(true);
            chr.debug_flags.set_disabled_updates(true);
        }
    }
}

fn clear_revenant_passive_buddy_slot(buddy_param_id: u32) {
    let Ok(param_repository) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return;
    };
    let Some(row) = param_repository.get_mut::<BuddyParam>(buddy_param_id) else {
        return;
    };
    row.set_npc_param_id(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
    row.set_npc_think_param_id(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
    row.set_npc_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
    row.set_npc_think_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
}

fn hide_revenant_native_summon_hud_if_revenant() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let active_effects = {
        let Some(player) = world_chr_man.main_player.as_ref() else {
            return;
        };
        player
            .chr_ins
            .special_effect
            .entries()
            .map(|entry| entry.param_id)
            .collect::<Vec<_>>()
    };
    if active_role_from_effects(&active_effects) != Some(Role::Revenant) {
        return;
    }

    let revenant_buddy_handles = revenant_alive_buddy_handles(world_chr_man);
    suppress_revenant_buddy_floating_gauges(world_chr_man);
    hide_revenant_native_summon_hud(&revenant_buddy_handles);
}

fn suppress_revenant_buddy_floating_gauges(world_chr_man: &mut WorldChrMan) {
    for group in world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|entry| entry.second.iter())
    {
        if !REVENANT_BUDDY_PARAM_IDS.contains(&(group.buddy_param_id as u32)) {
            continue;
        }

        let chr = unsafe { &mut *group.chr_ins.as_ptr() };
        if chr.field_ins_handle.is_empty() {
            continue;
        }

        chr.modules
            .action_flag
            .action_modifiers_flags
            .set_disable_floating_gauge_display(true);
    }
}

fn hide_revenant_native_summon_hud(revenant_buddy_handles: &[FieldInsHandle]) {
    let Ok(fe_man) = (unsafe { CSFeManImp::instance_mut() }) else {
        return;
    };

    hide_revenant_summon_msg(&mut fe_man.summon_msg_queue.current);
    for entry in &mut fe_man.summon_msg_queue.list.data {
        hide_revenant_summon_msg(&mut entry.data);
    }
    hide_revenant_summon_msg(&mut fe_man.summon_msg_queue.list.head.data);
    fe_man.frontend_values.summoned_spirit_ash_count = 0;
    for display in &mut fe_man.frontend_values.spirit_ash_display {
        display.suppress();
    }
    for display in &mut fe_man.friendly_chr_tag_displays {
        if revenant_friend_tag_should_hide(
            display.field_ins_handle,
            display.role_name_color,
            display.is_down_scaled,
            display.is_debug_summon,
            revenant_buddy_handles,
        ) {
            display.is_visible = false;
            display.is_not_on_screen = false;
            display.field_ins_handle = FieldInsHandle::none();
            display.hp = 0;
            display.max_recoverable_hp = 0;
            display.max_hp = 0;
            display.hp_max_uncapped = 0;
            display.last_damage_time_delta = 999.0;
        }
    }

    let front_end_values = unsafe { fe_man.front_end_view.front_end_view_values.as_mut() };
    front_end_values.summoned_spirit_ash_count = 0;
    for display in &mut front_end_values.spirit_ash_display {
        display.suppress();
    }
    for display in &mut fe_man.frontend_values.friendly_chr_tag_data {
        hide_revenant_frontend_tag(display, revenant_buddy_handles);
    }
    for display in &mut front_end_values.friendly_chr_tag_data {
        hide_revenant_frontend_tag(display, revenant_buddy_handles);
    }
}

fn hide_revenant_frontend_tag(display: &mut eldenring::cs::TagHudData, handles: &[FieldInsHandle]) {
    if !revenant_friend_tag_should_hide(
        display.field_ins_handle,
        display.role_name_color,
        display.is_down_scaled,
        false,
        handles,
    ) {
        return;
    }

    display.is_visible = false;
    display.update_position = false;
    display.is_not_on_screen = false;
    display.field_ins_handle = FieldInsHandle::none();
    display.hp = 0;
    display.hp_max_uncapped_difference = 0;
    display.hp_max_uncapped = 0;
    display.last_damage_taken = 0;
    display.last_hp_value = 0;
}

fn revenant_friend_tag_should_hide(
    handle: FieldInsHandle,
    role_name_color: u8,
    is_down_scaled: bool,
    _is_debug_summon: bool,
    handles: &[FieldInsHandle],
) -> bool {
    handles.iter().any(|known| *known == handle)
        || role_name_color == 1
        || is_down_scaled
}

fn hide_revenant_summon_msg(message: &mut eldenring::cs::SummonMsgData) {
    message.suppress();
}

fn revenant_alive_buddy_handles(world_chr_man: &WorldChrMan) -> Vec<FieldInsHandle> {
    let mut handles = revenant_buddy_group_handles(world_chr_man);
    for handle in world_chr_man
        .summon_buddy_chr_set
        .characters()
        .filter(|chr| revenant_buddy_hp_ratio(chr).is_some())
        .map(|chr| chr.field_ins_handle)
    {
        if !handles.iter().any(|known| known == &handle) {
            handles.push(handle);
        }
    }
    handles
}

fn revenant_buddy_group_handles(world_chr_man: &WorldChrMan) -> Vec<FieldInsHandle> {
    world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|entry| entry.second.iter())
        .filter_map(|group| {
            let chr = unsafe { group.chr_ins.as_ref() };
            if chr.field_ins_handle.is_empty() {
                None
            } else {
                Some(chr.field_ins_handle)
            }
        })
        .collect()
}

fn revenant_buddy_hp_ratio_by_slot(
    world_chr_man: &WorldChrMan,
    index: usize,
    handle: Option<FieldInsHandle>,
) -> Option<f32> {
    revenant_buddy_chr_by_param_id(world_chr_man, REVENANT_BUDDY_PARAM_IDS[index])
        .and_then(revenant_buddy_hp_ratio)
        .or_else(|| handle.and_then(|handle| revenant_buddy_hp_ratio_by_handle(world_chr_man, handle)))
}

fn revenant_buddy_chr_by_param_id(
    world_chr_man: &WorldChrMan,
    buddy_param_id: u32,
) -> Option<&ChrIns> {
    world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|entry| entry.second.iter())
        .find_map(|group| {
            if group.buddy_param_id != buddy_param_id as i32 {
                return None;
            }
            let chr = unsafe { group.chr_ins.as_ref() };
            if chr.field_ins_handle.is_empty() {
                None
            } else {
                Some(chr)
            }
        })
}

struct RevenantBuddyGroupChrMut<'a> {
    chr: &'a mut ChrIns,
    disappear_requested: bool,
}

fn revenant_buddy_group_by_param_id_mut(
    world_chr_man: &mut WorldChrMan,
    buddy_param_id: u32,
) -> Option<RevenantBuddyGroupChrMut<'_>> {
    world_chr_man
        .summon_buddy_manager
        .groups
        .iter()
        .flat_map(|entry| entry.second.iter())
        .find_map(|group| {
            if group.buddy_param_id != buddy_param_id as i32 {
                return None;
            }
            let chr = unsafe { &mut *group.chr_ins.as_ptr() };
            if chr.field_ins_handle.is_empty() {
                None
            } else {
                Some(RevenantBuddyGroupChrMut {
                    chr,
                    disappear_requested: group.disappear_requested,
                })
            }
        })
}

fn revenant_buddy_chr_by_handle_mut(
    world_chr_man: &mut WorldChrMan,
    handle: FieldInsHandle,
) -> Option<&mut ChrIns> {
    let chr_ptr = world_chr_man
        .summon_buddy_chr_set
        .chr_ins_by_handle_mut(&handle)
        .map(|chr| chr as *mut ChrIns)
        .or_else(|| {
            world_chr_man
                .chr_ins_by_handle_mut(&handle)
                .map(|chr| chr as *mut ChrIns)
        })?;
    Some(unsafe { &mut *chr_ptr })
}

fn revenant_buddy_chr_by_handle(
    world_chr_man: &WorldChrMan,
    handle: FieldInsHandle,
) -> Option<&ChrIns> {
    world_chr_man
        .summon_buddy_chr_set
        .chr_ins_by_handle(&handle)
        .or_else(|| world_chr_man.chr_ins_by_handle(&handle))
}

fn revenant_debug_chr_by_handle_mut(
    world_chr_man: &mut WorldChrMan,
    handle: FieldInsHandle,
) -> Option<&mut ChrIns> {
    let chr_ptr = world_chr_man
        .debug_chr_set
        .chr_ins_by_handle_mut(&handle)
        .map(|chr| chr as *mut ChrIns)
        .or_else(|| {
            world_chr_man
                .chr_ins_by_handle_mut(&handle)
                .map(|chr| chr as *mut ChrIns)
        })?;
    Some(unsafe { &mut *chr_ptr })
}

fn revenant_debug_chr_by_handle(
    world_chr_man: &WorldChrMan,
    handle: FieldInsHandle,
) -> Option<&ChrIns> {
    world_chr_man
        .debug_chr_set
        .chr_ins_by_handle(&handle)
        .or_else(|| world_chr_man.chr_ins_by_handle(&handle))
}

fn apply_revenant_buddy_hp_ratio(chr: &mut ChrIns, hp_ratio: f32) {
    let data = chr.modules.as_mut().data.as_mut();
    let max_hp = data.max_hp.max(1);
    data.hp = ((max_hp as f32) * hp_ratio.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, max_hp as f32) as i32;
}

fn revenant_buddy_param_npc_ids() -> [Option<i32>; REVENANT_SUMMON_COUNT] {
    let Ok(param_repository) = (unsafe { SoloParamRepository::instance() }) else {
        return [None; REVENANT_SUMMON_COUNT];
    };

    let mut npc_ids = [None; REVENANT_SUMMON_COUNT];
    for (index, param_id) in REVENANT_BUDDY_PARAM_IDS.into_iter().enumerate() {
        npc_ids[index] = param_repository
            .get::<BuddyParam>(param_id)
            .map(|row| row.npc_param_id())
            .filter(|npc_id| *npc_id > 0);
    }
    npc_ids
}

fn revenant_find_new_buddy_handle(
    world_chr_man: &WorldChrMan,
    target_npc_id: Option<i32>,
    current_handles: &[FieldInsHandle],
    baseline: &[FieldInsHandle],
    handles: &[Option<FieldInsHandle>; REVENANT_SUMMON_COUNT],
) -> Option<FieldInsHandle> {
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
            revenant_find_untracked_buddy_handle(world_chr_man, target_npc_id, current_handles, handles)
        })
}

fn revenant_find_untracked_buddy_handle(
    world_chr_man: &WorldChrMan,
    target_npc_id: Option<i32>,
    current_handles: &[FieldInsHandle],
    handles: &[Option<FieldInsHandle>; REVENANT_SUMMON_COUNT],
) -> Option<FieldInsHandle> {
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
    world_chr_man: &WorldChrMan,
    current_handles: &[FieldInsHandle],
    target_npc_id: Option<i32>,
    handles: &[Option<FieldInsHandle>; REVENANT_SUMMON_COUNT],
) -> Option<FieldInsHandle> {
    let target_npc_id = target_npc_id?;
    current_handles
        .iter()
        .copied()
        .find(|handle| {
            !handles.iter().flatten().any(|known| known == handle)
                && world_chr_man
                    .summon_buddy_chr_set
                    .chr_ins_by_handle(&handle)
                    .or_else(|| world_chr_man.chr_ins_by_handle(&handle))
                    .is_some_and(|chr| chr.npc_id == target_npc_id)
        })
}

fn revenant_buddy_hp_ratio_by_handle(
    world_chr_man: &WorldChrMan,
    handle: FieldInsHandle,
) -> Option<f32> {
    revenant_buddy_hp_ratio_from_fe(handle).or_else(|| {
        world_chr_man
            .summon_buddy_chr_set
            .chr_ins_by_handle(&handle)
            .and_then(revenant_buddy_hp_ratio)
            .or_else(|| world_chr_man.chr_ins_by_handle(&handle).and_then(revenant_buddy_hp_ratio))
    })
}

fn revenant_buddy_hp_ratio_from_fe(handle: FieldInsHandle) -> Option<f32> {
    if handle.is_empty() {
        return None;
    }

    let fe_man = unsafe { CSFeManImp::instance() }.ok()?;
    for display in &fe_man.frontend_values.spirit_ash_display {
        if display.field_ins_handle == handle {
            return hp_ratio_from_u32(display.hp, display.hp_max_uncapped);
        }
    }

    for display in &fe_man.friendly_chr_tag_displays {
        if display.field_ins_handle == handle {
            return hp_ratio_from_u32(display.hp, display.max_hp.max(display.hp_max_uncapped));
        }
    }

    fe_man
        .frontend_values
        .friendly_chr_tag_data
        .iter()
        .find(|display| display.field_ins_handle == handle)
        .and_then(|display| hp_ratio_from_u32(display.hp, display.hp_max_uncapped))
}

fn hp_ratio_from_u32(hp: u32, max_hp: u32) -> Option<f32> {
    if max_hp == 0 {
        return None;
    }
    Some((hp as f32 / max_hp as f32).clamp(0.0, 1.0))
}

fn revenant_buddy_hp_ratio(chr: &ChrIns) -> Option<f32> {
    if chr.field_ins_handle.is_empty() {
        return None;
    }
    revenant_buddy_hp_ratio_from_fe(chr.field_ins_handle).or_else(|| {
        let data = chr.modules.as_ref().data.as_ref();
        if data.max_hp <= 0 || data.hp < 0 {
            return None;
        }
        Some((data.hp as f32 / data.max_hp as f32).clamp(0.0, 1.0))
    })
}

fn collect_scholar_links(
    world_chr_man: &WorldChrMan,
    config: &RoleConfig,
    active_effects: &[i32],
) -> Vec<ScholarLink> {
    let mut links = Vec::new();
    if let Some(player) = world_chr_man.main_player.as_ref() {
        if active_effects.contains(&config.effect(SP_SCHOLAR_SYMPATHY_SELF)) {
            push_scholar_link(&mut links, &player.chr_ins, ScholarLinkKind::SelfLink);
        }
    }

    for entry in world_chr_man.chr_inses_by_distance.iter() {
        let chr = unsafe { entry.chr_ins.as_ref() };
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

fn push_scholar_link(links: &mut Vec<ScholarLink>, chr: &ChrIns, kind: ScholarLinkKind) {
    if links.iter().any(|link| link.handle == chr.field_ins_handle) {
        return;
    }
    links.push(ScholarLink {
        handle: chr.field_ins_handle,
        kind,
        hp: chr_data_hp(chr),
        last_hit_by: (!chr.last_hit_by.is_empty()).then_some(chr.last_hit_by),
    });
}

fn scholar_link_is_friendly(link: &ScholarLink) -> bool {
    matches!(link.kind, ScholarLinkKind::SelfLink | ScholarLinkKind::Ally)
}

fn scholar_counter_attacker(
    world_chr_man: &WorldChrMan,
    links: &[ScholarLink],
    victim: FieldInsHandle,
    last_hit_by: Option<FieldInsHandle>,
    locked_on_enemy: Option<FieldInsHandle>,
) -> Option<FieldInsHandle> {
    if let Some(attacker) = last_hit_by {
        if scholar_linked_enemy(links, attacker) {
            return Some(attacker);
        }
    }
    if let Some(locked_on_enemy) = locked_on_enemy {
        if scholar_linked_enemy(links, locked_on_enemy) {
            return Some(locked_on_enemy);
        }
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
    world_chr_man: &WorldChrMan,
    links: &[ScholarLink],
    victim: FieldInsHandle,
) -> Option<FieldInsHandle> {
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

fn scholar_chr_position_by_handle(
    world_chr_man: &WorldChrMan,
    handle: FieldInsHandle,
) -> Option<fromsoftware_shared::F32Vector4> {
    if let Some(player) = world_chr_man.main_player.as_ref() {
        if player.chr_ins.field_ins_handle == handle {
            return Some(player.chr_ins.chr_ctrl.as_ref().model_matrix.3);
        }
    }
    world_chr_man
        .chr_ins_by_handle(&handle)
        .map(|chr| chr.chr_ctrl.as_ref().model_matrix.3)
}

fn scholar_linked_enemy(links: &[ScholarLink], handle: FieldInsHandle) -> bool {
    links
        .iter()
        .any(|link| link.kind == ScholarLinkKind::Enemy && link.handle == handle)
}

fn scholar_scaled_amount(amount: i32, rate: f32) -> i32 {
    ((amount.max(0) as f32) * rate).floor() as i32
}

fn chr_has_speffect(chr: &ChrIns, sp_effect: i32) -> bool {
    chr.special_effect
        .entries()
        .any(|entry| entry.param_id == sp_effect)
}

fn chr_data_hp(chr: &ChrIns) -> i32 {
    chr.modules.as_ref().data.as_ref().hp
}

fn duchess_replay_snapshot(chr: &ChrIns) -> DuchessReplaySnapshot {
    DuchessReplaySnapshot {
        hp: chr_data_hp(chr),
        status: duchess_resist_values(chr),
        super_armor: chr.modules.as_ref().super_armor.as_ref().sa_durability,
    }
}

fn duchess_resist_values(chr: &ChrIns) -> [i32; 7] {
    let chr_ptr = chr as *const _ as usize;
    let module_bag = unsafe { *((chr_ptr + CHR_MODULE_BAG_OFFSET) as *const usize) };
    if module_bag == 0 {
        return [0; 7];
    }
    let resist_module = unsafe { *((module_bag + MODULE_RESIST_OFFSET) as *const usize) };
    if resist_module == 0 {
        return [0; 7];
    }

    let mut values = [0; 7];
    for (index, slot) in values.iter_mut().enumerate() {
        let value = unsafe { *((resist_module + 0x10 + index * 4) as *const i32) };
        *slot = if (0..1_000_000).contains(&value) {
            value
        } else {
            0
        };
    }
    values
}

fn hp_by_handle(world_chr_man: &WorldChrMan, handle: FieldInsHandle) -> Option<i32> {
    if let Some(player) = world_chr_man.main_player.as_ref() {
        if player.chr_ins.field_ins_handle == handle {
            return Some(chr_data_hp(&player.chr_ins));
        }
    }
    world_chr_man.chr_ins_by_handle(&handle).map(chr_data_hp)
}

fn apply_scholar_sympathy_damage_speffect(
    world_chr_man: &mut WorldChrMan,
    handle: FieldInsHandle,
) -> bool {
    if handle.is_empty() {
        return false;
    }
    let Some(target) = world_chr_man.chr_ins_by_handle_mut(&handle) else {
        return false;
    };
    target.apply_speffect(SP_SCHOLAR_SYMPATHY_DAMAGE, true);
    true
}

fn set_scholar_sympathy_speffect_damage(damage: i32) -> bool {
    if damage <= 0 {
        return false;
    }
    let Ok(param_repository) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return false;
    };
    let Some(sp_effect) =
        param_repository.get_mut::<SpEffectParam>(SP_SCHOLAR_SYMPATHY_DAMAGE as u32)
    else {
        return false;
    };
    sp_effect.set_change_hp_point(damage);
    true
}

fn set_duchess_replay_speffect(payload: DuchessReplayPayload) -> bool {
    if !payload.has_effect() {
        return false;
    }
    let Ok(param_repository) = (unsafe { SoloParamRepository::instance_mut() }) else {
        return false;
    };
    let Some(sp_effect) =
        param_repository.get_mut::<SpEffectParam>(SP_DUCHESS_REPLAY_DAMAGE as u32)
    else {
        return false;
    };
    sp_effect.set_change_hp_point(payload.hp_damage.max(0));
    sp_effect.set_poizon_attack_power(payload.status[0].max(0));
    sp_effect.set_disease_attack_power(payload.status[1].max(0));
    sp_effect.set_blood_attack_power(payload.status[2].max(0));
    sp_effect.set_curse_attack_power(payload.status[3].max(0));
    sp_effect.set_freeze_attack_power(payload.status[4].max(0));
    sp_effect.set_sleep_attack_power(payload.status[5].max(0));
    sp_effect.set_madness_attack_power(payload.status[6].max(0));
    sp_effect.set_change_super_armor_point(0.0);
    sp_effect.set_change_sa_point(0.0);
    true
}

fn patch_revenant_buddy_stones_once() {
    static APPLIED: AtomicBool = AtomicBool::new(false);

    if APPLIED.swap(true, Ordering::Relaxed) {
        return;
    }

    let Ok(param_repository) = (unsafe { SoloParamRepository::instance_mut() }) else {
        APPLIED.store(false, Ordering::Relaxed);
        return;
    };

    let mut row_count = 0usize;
    for (_, row) in param_repository.rows_mut::<BuddyStoneParam>() {
        row.set_eliminate_target_entity_id(0);
        row.set_summoned_event_flag_id(0);
        row.set_overwrite_activate_region_entity_id(0);
        row_count += 1;
    }
    for buddy_param_id in REVENANT_PASSIVE_BUDDY_PARAM_IDS {
        if let Some(row) = param_repository.get_mut::<BuddyParam>(buddy_param_id) {
            row.set_npc_param_id(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
            row.set_npc_think_param_id(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
            row.set_npc_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
            row.set_npc_think_param_id_ridden(REVENANT_PASSIVE_INVALID_BUDDY_NPC_ID);
        }
    }

    if row_count == 0 {
        APPLIED.store(false, Ordering::Relaxed);
        return;
    }

    append_damage_log(&format!(
        "[revenant] patched BuddyStoneParam summon restriction fields rows={row_count}"
    ));
}

fn apply_duchess_replay_posture_damage(chr: &mut ChrIns, payload: DuchessReplayPayload) {
    let damage = payload.super_armor_damage.max(0.0);
    if damage <= 0.0 {
        return;
    }

    let modules = chr.modules.as_mut();
    let toughness = modules.toughness.as_mut();
    toughness.toughness = (toughness.toughness - damage).max(0.0);

    let super_armor = modules.super_armor.as_mut();
    super_armor.sa_durability = (super_armor.sa_durability - damage).max(0.0);
}

fn scholar_damage_number_pos(
    world_chr_man: &WorldChrMan,
    handle: FieldInsHandle,
) -> Option<[f32; 2]> {
    world_chr_man
        .chr_ins_by_handle(&handle)
        .and_then(scholar_target_screen_pos)
}

fn apply_hp_delta_by_handle(
    world_chr_man: &mut WorldChrMan,
    handle: FieldInsHandle,
    delta: i32,
) -> i32 {
    if delta == 0 {
        return 0;
    }
    if let Some(player) = world_chr_man.main_player.as_mut() {
        if player.chr_ins.field_ins_handle == handle {
            return apply_chr_hp_delta(&mut player.chr_ins, delta);
        }
    }
    world_chr_man
        .chr_ins_by_handle_mut(&handle)
        .map(|chr| apply_chr_hp_delta(chr, delta))
        .unwrap_or(0)
}

fn apply_chr_hp_delta(chr: &mut ChrIns, delta: i32) -> i32 {
    let data = chr.modules.as_mut().data.as_mut();
    let old_hp = data.hp;
    let max_hp = data.max_hp.max(1);
    let new_hp = old_hp.saturating_add(delta).clamp(0, max_hp);
    data.hp = new_hp;
    new_hp - old_hp
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

fn consume_effect(
    player: &mut eldenring::cs::PlayerIns,
    active_effects: &[i32],
    sp_effect: i32,
) -> bool {
    if active_effects.contains(&sp_effect) {
        player.chr_ins.remove_speffect(sp_effect);
        true
    } else {
        false
    }
}

fn consume_event_effect(
    player: &mut eldenring::cs::PlayerIns,
    active_effects: &[i32],
    config: &RoleConfig,
    offset: i32,
) -> bool {
    let shared = shared_effect(offset);
    let legacy = config.effect(offset);
    if shared == legacy {
        return consume_effect(player, active_effects, shared);
    }

    let shared_active = consume_effect(player, active_effects, shared);
    let legacy_active = consume_effect(player, active_effects, legacy);
    shared_active || legacy_active
}

fn set_effect(player: &mut eldenring::cs::PlayerIns, sp_effect: i32, active: bool) {
    if active {
        player.apply_speffect(sp_effect, true);
    } else {
        player.chr_ins.remove_speffect(sp_effect);
    }
}

fn remove_role_outputs(player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
    for offset in [
        SP_SKILL_READY_1,
        SP_SKILL_READY_2,
        SP_ULTIMATE_READY_LEGACY_70,
        SP_ULTIMATE_READY_LEGACY_100,
        SP_ULTIMATE_READY,
    ] {
        player.chr_ins.remove_speffect(config.effect(offset));
        player.chr_ins.remove_speffect(shared_effect(offset));
    }
    if config.role == Role::Recluse {
        clear_recluse_outputs(player, config);
    } else if config.role == Role::Scholar {
        clear_scholar_self_outputs(player, config);
    }
}

fn clear_recluse_outputs(player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
    player
        .chr_ins
        .remove_speffect(config.effect(SP_RECLUSE_RELEASED));
    clear_recluse_absorb_outputs(player, config);
    clear_recluse_magic_outputs(player, config);
}

fn clear_recluse_absorb_outputs(player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
    for offset in [
        SP_RECLUSE_RESTORE_FP,
        SP_RECLUSE_ABSORB_MAGIC,
        SP_RECLUSE_ABSORB_FIRE,
        SP_RECLUSE_ABSORB_LIGHTNING,
        SP_RECLUSE_ABSORB_HOLY,
    ] {
        player.chr_ins.remove_speffect(config.effect(offset));
    }
}

fn clear_recluse_magic_outputs(player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
    for index in 0..14 {
        player
            .chr_ins
            .remove_speffect(config.effect(SP_RECLUSE_MIXED_MAGIC_BASE + index));
    }
}

fn clear_scholar_self_outputs(player: &mut eldenring::cs::PlayerIns, config: &RoleConfig) {
    for index in 0..3 {
        player
            .chr_ins
            .remove_speffect(config.effect(SP_SCHOLAR_SELF_STAGE_BASE + index));
    }
}

fn clear_scholar_enemy_outputs(enemy: &mut eldenring::cs::ChrIns, config: &RoleConfig) {
    for index in 0..3 {
        enemy.remove_speffect(config.effect(SP_SCHOLAR_ENEMY_STAGE_BASE + index));
    }
}

fn remove_all_role_outputs(player: &mut eldenring::cs::PlayerIns) {
    for config in ROLE_CONFIGS.iter() {
        remove_role_outputs(player, config);
    }
}

