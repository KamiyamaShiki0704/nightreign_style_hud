#[unsafe(no_mangle)]
/// # Safety
pub unsafe extern "C" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    revenant_cleanup::MODULE.store(hmodule.0 as usize, Ordering::Relaxed);
    debug::initialize_checked::<ImguiDx12Hooks>(
        hmodule,
        reason,
        || {
            eldenring::initialize_game_bindings()?;
            wait_for_system_init(&Program::current(), Duration::MAX)
                .map_err(|error| error.to_string())
        },
        NightreignStyleHud::new(),
    )
}

#[derive(Clone, Copy)]
struct ChargeSlot {
    label: &'static str,
    accent: ImColor32,
    icon_scale: f32,
    ready_glow: f32,
}

struct NightreignStyleHud {
    scale: f32,
    icons: IconSet,
    skill: ChargeSlot,
    ultimate: ChargeSlot,
    charges: ChargeState,
    animation_start: Instant,
}

impl NightreignStyleHud {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            scale: 1.0,
            icons: IconSet::default(),
            charges: ChargeState::new(now),
            animation_start: now,
            skill: ChargeSlot {
                label: "SKILL",
                accent: ImColor32::from_rgba(80, 154, 255, 255),
                icon_scale: 0.55,
                ready_glow: 0.35,
            },
            ultimate: ChargeSlot {
                label: "ART",
                accent: ImColor32::from_rgba(235, 214, 150, 255),
                icon_scale: 1.0,
                ready_glow: 1.0,
            },
        }
    }

    fn update_scale(&mut self, viewport: [f32; 2]) {
        self.scale = hud_scale_for_viewport(viewport);
    }
}

const HUD_REFERENCE_WIDTH: f32 = 2560.0;
const HUD_REFERENCE_HEIGHT: f32 = 1440.0;
const HUD_LEGACY_4K_SCALE: f32 = 1.5;

fn hud_scale_for_viewport(viewport: [f32; 2]) -> f32 {
    let [width, height] = viewport;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return 1.0;
    }

    (width / HUD_REFERENCE_WIDTH).min(height / HUD_REFERENCE_HEIGHT)
}

fn scale_legacy_4k_pixels(value: f32, scale: f32) -> f32 {
    value * scale / HUD_LEGACY_4K_SCALE
}

impl ImguiRenderLoop for NightreignStyleHud {
    fn initialize(&mut self, ctx: &mut Context, render_context: &mut dyn RenderContext) {
        ctx.io_mut().config_flags |= hudhook::imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
        self.icons.load(render_context);
        install_damage_hook_once();
        install_undertaker_auto_dodge_hook_once();
        install_revenant_summon_range_hook_once();
        install_camera_directed_position_hook_once();
        install_multi_lock_bullet_hook_once();
        install_ironeye_runtime_tasks_once();
    }

    fn render(&mut self, ui: &mut Ui) {
        let Ok(_native_access) = revenant_cleanup::ACCESS.lock() else { return };
        revenant_cleanup::observe_session();
        let viewport = ui.io().display_size;
        self.update_scale(viewport);

        let now = Instant::now();
        let game_state = hud_game_state(now);
        if game_state.loading_cycle_changed {
            self.charges.begin_map_refresh();
        }
        self.charges.observe_revenant_cleanup_generation();
        if game_state.hidden {
            return;
        }
        update_multi_lock_live_bullets();
        let _ = multi_lock_snapshot_targets();
        let snapshot = self.charges.sync(now);
        draw_resurrection_accumulation_ui(
            ui,
            self.charges.resurrection_accumulation_snapshot(),
            self.charges
                .teammate_resurrection_accumulation_snapshots(),
            self.icons.resurrection_accumulation,
            self.scale,
        );
        let Some(snapshot) = snapshot else {
            if STANDALONE_PRECISION_AIMING.load(Ordering::Relaxed) {
                draw_standalone_precision_reticle_active(
                    ui,
                    &self.icons.standalone_precision,
                    self.scale,
                );
            } else if IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed)
                && !PRECISION_CALIBRATION_AIMING.load(Ordering::Relaxed)
            {
                draw_ironeye_precision_reticle_active(ui, &self.icons.ironeye, self.scale);
            }
            draw_recluse_blood_debug(self.charges.recluse_blood_debug(), self.scale);
            return;
        };
        let role = snapshot.role;

        let ultimate_radius = 70.0 * self.scale;
        let skill_radius = ultimate_radius * 0.7;
        let gap = 44.0 * self.scale;
        let group_width = skill_radius * 2.0 + gap + ultimate_radius * 2.0;
        let left_edge = viewport[0] * 0.5 - group_width * 0.5;
        let y = viewport[1] - (ultimate_radius + 96.0 * self.scale);
        let skill_center = [left_edge + skill_radius, y];
        let ultimate_center = [left_edge + skill_radius * 2.0 + gap + ultimate_radius, y];
        let role_icons = self.icons.role(role);
        let skill_icon = if role == Role::Recluse {
            snapshot
                .recluse_ready_magic
                .and_then(|index| self.icons.recluse.magic_region(index - 1))
                .or(role_icons.skill)
        } else {
            role_icons.skill
        };
        let skill_icon_scale = if role == Role::Recluse && snapshot.recluse_ready_magic.is_some() {
            1.65
        } else {
            1.0
        };

        draw_charge_slot(
            ui,
            skill_center,
            skill_radius,
            self.skill,
            snapshot.skill_layers,
            snapshot.skill_ready,
            snapshot.skill_top_down,
            snapshot.skill_count,
            skill_icon,
            skill_icon_scale,
            None,
            if snapshot.undertaker_enhanced_skill_buff {
                self.icons.effects.undertaker_skill()
            } else if snapshot.undertaker_normal_skill_buff {
                self.icons.effects.ultimate_on()
            } else {
                [None, None, None]
            },
            false,
            ActiveTextureStyle::Default,
            snapshot.hud_solid,
            now,
            self.animation_start,
        );
        if role == Role::Recluse {
            draw_recluse_skill_ui(
                ui,
                skill_center,
                skill_radius,
                &snapshot,
                &self.icons.recluse,
            );
        } else if role == Role::Scholar {
            draw_scholar_observation_ui(
                ui,
                &snapshot,
                &self.icons.scholar,
                now,
                self.animation_start,
                self.scale,
            );
        } else if role == Role::Ironeye {
            draw_ironeye_weakness_marks(
                ui,
                &snapshot,
                &self.icons.ironeye,
                now,
                self.animation_start,
                self.scale,
            );
        } else if role == Role::Wylder {
            draw_wylder_skill_lock_ui(
                ui,
                &snapshot,
                self.icons.wylder_skill_lock,
                self.scale,
            );
        } else if role == Role::Revenant && !snapshot.revenant_skill_move_hud {
            draw_revenant_static_summon_ui(
                ui,
                skill_center,
                skill_radius,
                &snapshot,
                &self.icons.revenant,
                self.scale,
            );
        } else if role == Role::Revenant {
            draw_revenant_skill_move_summon_ui(
                ui,
                viewport,
                &snapshot,
                &self.icons.revenant,
                self.scale,
            );
        }
        if snapshot.standalone_precision_aiming {
            draw_standalone_precision_reticle(
                ui,
                &snapshot,
                &self.icons.standalone_precision,
                self.scale,
            );
        } else {
            draw_ironeye_precision_reticle(ui, &snapshot, &self.icons.ironeye, self.scale);
        }
        if role == Role::Scholar {
            draw_scholar_damage_numbers(ui, &snapshot, self.scale);
        }
        draw_undertaker_auto_dodge_probe(snapshot.undertaker_enhanced_skill_buff, self.scale);
        draw_recluse_blood_debug(snapshot.recluse_blood_debug, self.scale);
        draw_charge_slot(
            ui,
            ultimate_center,
            ultimate_radius,
            self.ultimate,
            [snapshot.ultimate_progress, 0.0],
            snapshot.ultimate_ready,
            snapshot.ultimate_top_down,
            None,
            role_icons.ultimate,
            1.0,
            if snapshot.undertaker_free_ultimate_active {
                None
            } else {
                self.icons.effects.ultimate_on_region(0)
            },
            if snapshot.undertaker_free_ultimate_active {
                self.icons.effects.undertaker_ultimate()
            } else if snapshot.executor_ultimate_active {
                self.icons.effects.ultimate_on()
            } else if snapshot.ultimate_ready {
                [None, self.icons.effects.ultimate_on_region(1), None]
            } else {
                [None, None, None]
            },
            snapshot.undertaker_free_ultimate_active,
            if snapshot.undertaker_free_ultimate_active {
                ActiveTextureStyle::UndertakerFreeUltimate
            } else {
                ActiveTextureStyle::Default
            },
            snapshot.hud_solid,
            now,
            self.animation_start,
        );
        if snapshot.revenant_skill_move_hud {
            draw_charge_slot_dim_overlay(ui, ultimate_center, ultimate_radius, self.scale);
        }
    }
}

struct LoadingHudGate {
    last_menu_load_counter: Option<i32>,
    hide_until: Instant,
}

#[derive(Clone, Copy, Default)]
struct HudGameState {
    hidden: bool,
    loading_cycle_changed: bool,
}

#[cfg(test)]
mod map_refresh_regression_tests {
    use super::*;

    #[test]
    fn normal_game_hud_does_not_require_revenant_cleanup_session() {
        let now = Instant::now();
        let mut gate = LoadingHudGate { last_menu_load_counter: None, hide_until: now };
        // The production render decision takes no cleanup-availability input.
        assert!(!gate.observe_state(Some(2), Some(false), false, now).hidden,
            "normal gameplay HUD disappeared because passive cleanup reported no session");
    }

    #[test]
    fn refresh_clears_recluse_slots_and_prepared_magic() {
        let now = Instant::now();
        let mut state = ChargeState::new(now);
        state.reset_for_role(Some(Role::Recluse));
        state.push_recluse_element(RecluseElement::Magic);
        state.push_recluse_element(RecluseElement::Fire);
        state.push_recluse_element(RecluseElement::Holy);
        state.recluse_ready_magic = state.recluse_magic_index();
        assert!(state.recluse_ready_magic.is_some());
        state.begin_map_refresh();
        assert!(state.recluse_elements.iter().all(Option::is_none),
            "refresh retained filled Recluse slots");
        assert!(state.recluse_ready_magic.is_none());
    }

    #[test]
    fn long_loading_stays_hidden_then_restores_after_completion() {
        let now = Instant::now();
        let mut gate = LoadingHudGate { last_menu_load_counter: None, hide_until: now };
        assert!(!gate.observe_state(Some(2), Some(false), false, now).hidden);
        let begin = gate.observe_state(Some(3), Some(true), false, now);
        assert!(begin.hidden && begin.loading_cycle_changed);
        // The captured load lasted ~3.8s. The native FE state is deliberately
        // permissive here: the loading request alone must hide beyond 1.5s.
        let during = gate.observe_state(Some(3), Some(true), false, now + Duration::from_secs(3));
        assert!(during.hidden && !during.loading_cycle_changed);
        assert!(!gate.observe_state(Some(3), Some(false), false, now + Duration::from_secs(4)).hidden);
    }

    #[test]
    fn initial_observation_during_loading_is_hidden_without_counter_change() {
        let now = Instant::now();
        let mut gate = LoadingHudGate { last_menu_load_counter: None, hide_until: now };
        let state = gate.observe_state(Some(3), Some(true), false, now);
        assert!(state.hidden);
        assert!(!state.loading_cycle_changed);
    }

    #[test]
    fn ordinary_menu_keeps_elements_and_does_not_request_map_cleanup() {
        let now = Instant::now();
        let mut gate = LoadingHudGate { last_menu_load_counter: Some(3), hide_until: now };
        let mut charges = ChargeState::new(now);
        charges.push_recluse_element(RecluseElement::Fire);
        for menu_open in [true, false] {
            let state = gate.observe_state(Some(3), Some(false), menu_open, now);
            if state.loading_cycle_changed { charges.begin_map_refresh(); }
            assert_eq!(state.hidden, menu_open);
            assert!(charges.recluse_elements == [Some(RecluseElement::Fire), None, None]);
            assert!(!charges.recluse_map_refresh_cleanup_pending);
        }
    }

    #[test]
    fn refresh_clears_partial_slots_and_old_target_then_allows_refill() {
        let now = Instant::now();
        for count in 1..=3 {
            let mut state = ChargeState::new(now);
            for _ in 0..count { state.push_recluse_element(RecluseElement::Magic); }
            state.recluse_lock_element = Some(RecluseElement::Magic);
            state.recluse_lock_damage_module = Some(0x1234);
            record_recluse_damage_element(0x1234, RecluseElement::Magic);
            state.recluse_outputs_clear_at = Some(now + Duration::from_secs(5));
            state.revenant_summon_hp = [0.25, 0.5, 0.75];
            state.begin_map_refresh();
            assert!(state.recluse_elements == [None; 3]);
            assert!(state.recluse_lock_element.is_none());
            assert!(state.recluse_lock_damage_module.is_none());
            assert!(lookup_recluse_damage_element(0x1234).is_none());
            assert!(state.recluse_outputs_clear_at.is_none());
            assert!(state.recluse_map_refresh_cleanup_pending);
            assert_eq!(state.revenant_summon_hp, [0.25, 0.5, 0.75]);
            state.push_recluse_element(RecluseElement::Fire);
            assert!(state.recluse_elements == [Some(RecluseElement::Fire), None, None]);
        }
    }
}

impl LoadingHudGate {
    fn observe_state(&mut self, counter: Option<i32>, loading: Option<bool>, native_hidden: bool, now: Instant) -> HudGameState {
        let loading_cycle_changed = counter.is_some_and(|counter| self.observe_menu_load_counter(counter, now));
        HudGameState {
            hidden: loading == Some(true) || native_hidden || now < self.hide_until,
            loading_cycle_changed,
        }
    }

    fn observe_menu_load_counter(&mut self, counter: i32, now: Instant) -> bool {
        match self.last_menu_load_counter {
            Some(previous) if previous != counter => {
                self.last_menu_load_counter = Some(counter);
                self.hide_until = now + Duration::from_secs_f32(HUD_LOADING_HIDE_GRACE_SECONDS);
                true
            }
            None => {
                self.last_menu_load_counter = Some(counter);
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod hud_resolution_scale_tests {
    use super::hud_scale_for_viewport;

    fn assert_scale(viewport: [f32; 2], expected: f32) {
        let actual = hud_scale_for_viewport(viewport);
        assert!(
            (actual - expected).abs() <= 0.001,
            "viewport={viewport:?} expected={expected} actual={actual}"
        );
    }

    #[test]
    fn standard_resolutions_match_the_1440p_reference() {
        assert_scale([3840.0, 2160.0], 1.5);
        assert_scale([2560.0, 1440.0], 1.0);
        assert_scale([1920.0, 1080.0], 0.75);
        assert_scale([1280.0, 720.0], 0.5);
        assert_scale([960.0, 540.0], 0.375);
    }

    #[test]
    fn ultrawide_resolution_is_limited_by_height() {
        assert_scale([3440.0, 1440.0], 1.0);
        assert_scale([5120.0, 1440.0], 1.0);
    }

    #[test]
    fn invalid_viewports_use_a_safe_default() {
        assert_scale([0.0, 1440.0], 1.0);
        assert_scale([2560.0, -1.0], 1.0);
        assert_scale([f32::NAN, 1440.0], 1.0);
        assert_scale([2560.0, f32::INFINITY], 1.0);
    }
}

static LOADING_HUD_GATE: LazyLock<Mutex<LoadingHudGate>> = LazyLock::new(|| {
    Mutex::new(LoadingHudGate {
        last_menu_load_counter: None,
        hide_until: Instant::now(),
    })
});

const HUD_LOADING_HIDE_GRACE_SECONDS: f32 = 1.5;

fn hud_game_state(now: Instant) -> HudGameState {
    let counter = unsafe { CSNowLoadingHelper::instance() }.ok().map(|loading| loading.menu_load_counter);
    let loading = loading_state::screen_active();
    let native_hidden = unsafe { CSFeManImp::instance() }
        .map(|fe_man| fe_man.hud_state != CSFeManHudState::Default)
        .unwrap_or(false);
    if let Ok(mut gate) = LOADING_HUD_GATE.lock() {
        return gate.observe_state(counter, loading, native_hidden, now);
    }
    HudGameState { hidden: true, loading_cycle_changed: false }
}

#[cfg(test)]
mod loading_hud_gate_tests {
    use super::*;

    #[test]
    fn first_counter_initializes_without_reporting_a_refresh() {
        let now = Instant::now();
        let mut gate = LoadingHudGate {
            last_menu_load_counter: None,
            hide_until: now,
        };

        assert!(!gate.observe_menu_load_counter(10, now));
        assert_eq!(gate.last_menu_load_counter, Some(10));
    }

    #[test]
    fn changed_counter_reports_one_refresh_and_starts_grace_period() {
        let now = Instant::now();
        let mut gate = LoadingHudGate {
            last_menu_load_counter: Some(10),
            hide_until: now,
        };

        assert!(gate.observe_menu_load_counter(11, now));
        assert!(gate.hide_until > now);
        assert!(!gate.observe_menu_load_counter(11, now));
    }
}

