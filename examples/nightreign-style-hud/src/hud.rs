#[unsafe(no_mangle)]
/// # Safety
pub unsafe extern "C" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    debug::initialize::<ImguiDx12Hooks>(
        hmodule,
        reason,
        || {
            wait_for_system_init(&Program::current(), Duration::MAX)
                .expect("Timeout waiting for system init");
        },
        NightreignStyleHud::new(),
    )
}

#[derive(Clone, Copy)]
struct ChargeSlot {
    label: &'static str,
    accent: ImColor32,
    fill: ImColor32,
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
                fill: ImColor32::from_rgba(80, 154, 255, 122),
                icon_scale: 0.55,
                ready_glow: 0.35,
            },
            ultimate: ChargeSlot {
                label: "ART",
                accent: ImColor32::from_rgba(235, 214, 150, 255),
                fill: ImColor32::from_rgba(80, 154, 255, 122),
                icon_scale: 1.0,
                ready_glow: 1.0,
            },
        }
    }

    fn update_scale(&mut self) {
        if let Ok(window) = unsafe { CSWindowImp::instance() } {
            self.scale = (window.screen_width as f32 / 1920.0).clamp(0.65, 1.5);
        }
    }
}

impl ImguiRenderLoop for NightreignStyleHud {
    fn initialize(&mut self, ctx: &mut Context, render_context: &mut dyn RenderContext) {
        ctx.io_mut().config_flags |= hudhook::imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
        self.icons.load(render_context);
        install_damage_hook_once();
        install_revenant_summon_range_hook_once();
        install_camera_directed_position_hook_once();
        install_multi_lock_bullet_hook_once();
        install_ironeye_runtime_tasks_once();
    }

    fn render(&mut self, ui: &mut Ui) {
        self.update_scale();

        let now = Instant::now();
        if should_hide_hud_for_game_state(now) {
            return;
        }
        update_multi_lock_live_bullets();
        let _ = multi_lock_snapshot_targets();
        let Some(snapshot) = self.charges.sync(now) else {
            return;
        };
        let role = snapshot.role;

        let viewport = ui.io().display_size;
        let ultimate_radius = 56.0 * self.scale;
        let skill_radius = ultimate_radius * 0.7;
        let gap = 38.0 * self.scale;
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
            );
        } else if role == Role::Ironeye {
            draw_ironeye_precision_reticle(ui, &snapshot, &self.icons.ironeye, self.scale);
            draw_ironeye_weakness_marks(
                ui,
                &snapshot,
                &self.icons.ironeye,
                now,
                self.animation_start,
            );
        } else if role == Role::Wylder {
            draw_wylder_skill_lock_ui(
                ui,
                &snapshot,
                self.icons.wylder_skill_lock,
                self.scale,
            );
        } else if role == Role::Revenant {
            draw_revenant_static_summon_ui(
                ui,
                skill_center,
                skill_radius,
                &snapshot,
                &self.icons.revenant,
                self.scale,
            );
        }
        if role == Role::Scholar {
            draw_scholar_damage_numbers(ui, &snapshot);
        }
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
            now,
            self.animation_start,
        );
    }
}

struct LoadingHudGate {
    last_menu_load_counter: Option<i32>,
    hide_until: Instant,
}

static LOADING_HUD_GATE: LazyLock<Mutex<LoadingHudGate>> = LazyLock::new(|| {
    Mutex::new(LoadingHudGate {
        last_menu_load_counter: None,
        hide_until: Instant::now(),
    })
});

const HUD_LOADING_HIDE_GRACE_SECONDS: f32 = 1.5;

fn should_hide_hud_for_game_state(now: Instant) -> bool {
    if unsafe { CSFeManImp::instance() }
        .map(|fe_man| fe_man.hud_state != CSFeManHudState::Default)
        .unwrap_or(false)
    {
        return true;
    }

    if let Ok(loading) = unsafe { CSNowLoadingHelper::instance() } {
        if let Ok(mut gate) = LOADING_HUD_GATE.lock() {
            match gate.last_menu_load_counter {
                Some(previous) if previous != loading.menu_load_counter => {
                    gate.last_menu_load_counter = Some(loading.menu_load_counter);
                    gate.hide_until = now + Duration::from_secs_f32(HUD_LOADING_HIDE_GRACE_SECONDS);
                }
                None => {
                    gate.last_menu_load_counter = Some(loading.menu_load_counter);
                }
                _ => {}
            }
            return now < gate.hide_until;
        }
    }

    false
}

