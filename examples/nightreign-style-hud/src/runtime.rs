static INSTALL_IRONEYE_RUNTIME_TASKS: Once = Once::new();
static IRONEYE_MANUAL_AIMING: AtomicBool = AtomicBool::new(false);
static STANDALONE_PRECISION_AIMING: AtomicBool = AtomicBool::new(false);
static PRECISION_CALIBRATION_AIMING: AtomicBool = AtomicBool::new(false);
const IRONEYE_VISUAL_FOV_SCALE: f32 = 1.22;
const IRONEYE_VISUAL_CAMERA_PULLBACK: f32 = 2.15;
const IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET: f32 = -0.70;
const IRONEYE_RETICLE_CONVERGENCE_DISTANCE: f32 = 30.0;

fn install_ironeye_runtime_tasks_once() {
    INSTALL_IRONEYE_RUNTIME_TASKS.call_once(|| {
        if let Err(error) = sdk_precision::configure(
            sdk_precision::PrecisionSettings {
                fov_scale: IRONEYE_VISUAL_FOV_SCALE,
                pullback: IRONEYE_VISUAL_CAMERA_PULLBACK,
                right_offset: IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET,
            },
            |effects| sdk_precision::ActivationPolicy {
                enabled: active_role_from_effects(effects) == Some(Role::Ironeye)
                    || effects.contains(&SP_STANDALONE_PRECISION_AIM)
                    || effects.contains(&SP_PRECISION_CALIBRATION),
                forced_aim: effects.contains(&SP_PRECISION_CALIBRATION),
                standalone: effects.contains(&SP_STANDALONE_PRECISION_AIM),
                calibration: effects.contains(&SP_PRECISION_CALIBRATION),
            },
        ) {
            append_damage_log(&format!("[init] precision configuration rejected: {error}"));
            return;
        }

        if let Err(error) = native_tasks::install([
            ironeye_pre_behavior_task,
            ironeye_pre_camera_task,
            ironeye_post_camera_task,
            ironeye_sound_precision_task,
            ironeye_physics_precision_task,
            camera_directed_motion_task,
            ironeye_damage_precision_task,
            ironeye_post_damage_task,
            revenant_menu_man_task,
            ironeye_post_scaleform_task,
            ironeye_draw_pre_task,
        ]) {
            append_damage_log(&format!(
                "[init] Shiki HUD registration failed, partial owners retained: {error}"
            ));
            return;
        }
        append_damage_log("[init] ironeye runtime tasks installed");
    });
}

fn ironeye_pre_behavior_task() {
    update_undertaker_auto_dodge_before_behavior();
    restore_ironeye_precision_before_behavior();
}

fn ironeye_pre_camera_task() {
    suppress_ironeye_precision_before_camera();
}

fn ironeye_post_camera_task() {
    update_ironeye_precision_view_after_camera();
}

fn ironeye_sound_precision_task() {
    hide_revenant_native_summon_hud_if_revenant();
    advance_ironeye_precision(3);
}
fn ironeye_physics_precision_task() {
    hide_revenant_native_summon_hud_if_revenant();
    advance_ironeye_precision(4);
}
fn ironeye_damage_precision_task() {
    hide_revenant_native_summon_hud_if_revenant();
    advance_ironeye_precision(5);
}

fn camera_directed_motion_task() {
    apply_camera_directed_position_delta_if_active();
}

fn ironeye_post_damage_task() {
    hide_revenant_native_summon_hud_if_revenant();
    suppress_ironeye_precision_before_menu();
}

fn revenant_menu_man_task() {
    hide_revenant_native_summon_hud_if_revenant();
}

fn ironeye_post_scaleform_task() {
    hide_revenant_native_summon_hud_if_revenant();
    finish_ironeye_precision_frame_after_scaleform();
}

fn ironeye_draw_pre_task() {
    hide_revenant_native_summon_hud_if_revenant();
    restore_ironeye_visual_camera_for_draw();
}

fn advance_ironeye_precision(phase: usize) {
    match sdk_precision::advance(phase) {
        Ok(state) => {
            IRONEYE_MANUAL_AIMING.store(state.active, Ordering::Relaxed);
            STANDALONE_PRECISION_AIMING.store(state.active && state.standalone, Ordering::Relaxed);
            PRECISION_CALIBRATION_AIMING
                .store(state.active && state.calibration, Ordering::Relaxed);
        }
        Err(error) => {
            IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
            STANDALONE_PRECISION_AIMING.store(false, Ordering::Relaxed);
            PRECISION_CALIBRATION_AIMING.store(false, Ordering::Relaxed);
            append_damage_log(&format!("Shiki precision phase {phase} rejected: {error}"));
        }
    }
}
fn restore_ironeye_precision_before_behavior() {
    advance_ironeye_precision(0);
}
fn suppress_ironeye_precision_before_camera() {
    advance_ironeye_precision(1);
}
fn update_ironeye_precision_view_after_camera() {
    advance_ironeye_precision(2);
}
fn suppress_ironeye_precision_before_menu() {
    advance_ironeye_precision(6);
}
fn finish_ironeye_precision_frame_after_scaleform() {
    advance_ironeye_precision(7);
}
fn restore_ironeye_visual_camera_for_draw() {
    advance_ironeye_precision(8);
}
fn ironeye_camera_debug_lines() -> Option<[String; 3]> {
    if !IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed) {
        return None;
    }
    sdk_host::with_objects(|objects, epoch| {
        let Some(player) = objects.primary_player_identity(epoch)? else {
            return Ok(None);
        };
        let Some(cameras) = objects.cameras()? else {
            return Ok(None);
        };
        let effects = objects.character_effects(player, 4096)?;
        if !effects.is_some_and(|v| active_role_from_effects(&v.entries.iter().map(|entry| entry.native_id).collect::<Vec<_>>()) == Some(Role::Ironeye)) {
            return Ok(None);
        }
        let input = objects.character_aim_input(player)?;
        let flags = objects.character_flags(player)?;
        let policy = objects.controller_policy(player)?;
        let mut perspectives = [None; 4];
        for (slot, target) in perspectives.iter_mut().zip(cameras.perspectives) {
            if let Some(target) = target { *slot = objects.perspective_camera(target.address)?; }
        }
        let fovs = perspectives.map(|v| v.map_or(f32::NAN, |v| f32::from_bits(v.frustum_bits[0])));
        let cs_line = format!(
            "Ironeye cam flags m:{} p:{} hks:{} mask:{:02X} cs fov {:.3}/{:.3}/{:.3}/{:.3}",
            input.is_some_and(|v| v.manual_aim_input_admitted()) as u8,
            flags.is_some_and(|v| v.precision_aim_requested()) as u8,
            policy.and_then(|v| v.modifier_word1c).is_some_and(|v| v & (1 << 13) != 0) as u8,
            cameras.native_flags, fovs[0], fovs[1], fovs[2], fovs[3],
        );
        let chr_line = if let Some(chr) = objects.character_cameras()? {
            let mut values = [None; 4];
            for (slot, target) in values.iter_mut().zip(chr.cameras) {
                if let Some(target) = target { *slot = objects.perspective_camera(target.address)?; }
            }
            let fov = values.map(|v| v.map_or(f32::NAN, |v| f32::from_bits(v.frustum_bits[0])));
            let distance = |a: Option<shikifsrs_elden_ring::cameras::PerspectiveCameraObservation>, b: Option<shikifsrs_elden_ring::cameras::PerspectiveCameraObservation>| {
                match (a, b) {
                    (Some(a), Some(b)) => a.matrix_bits[3][..3].iter().zip(&b.matrix_bits[3][..3]).map(|(a,b)| (f32::from_bits(*a) - f32::from_bits(*b)).powi(2)).sum::<f32>().sqrt(),
                    _ => f32::NAN,
                }
            };
            format!("Ironeye chr typed slots {:?} fov p/ex/aim/dist {:.3}/{:.3}/{:.3}/{:.3} d p-ex:{:.2} aim-ex:{:.2} cs-ex:{:.2}",
                chr.cameras.map(|v| v.map(|v| v.kind)), fov[0], fov[1], fov[2], fov[3],
                distance(values[0], values[1]), distance(values[2], values[1]), distance(perspectives[0], values[1]))
        } else { "Ironeye chr cam unavailable".to_string() };
        let game_line = if let Some(game) = objects.game_camera_overrides(epoch)? {
            let [dist,lerp,progress,duration,zoom_lerp] = game.payload.scalar_bits.map(f32::from_bits);
            let [reset,collision] = game.payload.flag_bytes;
            format!("Ironeye game zoom dist:{dist:.3} lerp:{lerp:.3} prog:{progress:.3} dur:{duration:.3} zlerp:{zoom_lerp:.3} reset:{reset} coll:{collision}")
        } else { "Ironeye game cam unavailable".to_string() };
        Ok(Some([cs_line, chr_line, game_line]))
    })
    .ok()
    .flatten()
}

fn ironeye_precision_reticle_screen_pos(viewport: [f32; 2]) -> [f32; 2] {
    let center = [viewport[0] * 0.5, viewport[1] * 0.5];
    let Ok(camera) = native_camera::HudCamera::observe() else {
        return center;
    };
    let cam = camera.pers_cam_1.as_ref();
    let forward = cam.forward();
    let right = cam.right();
    let len = (forward.0 * forward.0 + forward.1 * forward.1 + forward.2 * forward.2).sqrt();
    let right_len = (right.0 * right.0 + right.1 * right.1 + right.2 * right.2).sqrt();
    if !len.is_finite() || len < 0.001 || !right_len.is_finite() || right_len < 0.001 {
        return center;
    }

    let fx = forward.0 / len;
    let fy = forward.1 / len;
    let fz = forward.2 / len;
    let rx = right.0 / right_len;
    let ry = right.1 / right_len;
    let rz = right.2 / right_len;
    let display_delta = [
        fx * IRONEYE_VISUAL_CAMERA_PULLBACK + rx * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET,
        fy * IRONEYE_VISUAL_CAMERA_PULLBACK + ry * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET,
        fz * IRONEYE_VISUAL_CAMERA_PULLBACK + rz * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET,
    ];
    let matrix = cam.matrix;
    let aim_point = native_camera::Vector4(
        matrix.3.0 + display_delta[0] + fx * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.1 + display_delta[1] + fy * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.2 + display_delta[2] + fz * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        1.0,
    );

    project_lock_on_position(aim_point, cam).unwrap_or(center)
}
