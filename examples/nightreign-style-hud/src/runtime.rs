static INSTALL_IRONEYE_RUNTIME_TASKS: Once = Once::new();
static IRONEYE_MANUAL_AIMING: AtomicBool = AtomicBool::new(false);
static IRONEYE_SUPPRESSED_PRECISION: AtomicBool = AtomicBool::new(false);
static IRONEYE_SUPPRESSED_HKS_PRECISION: AtomicBool = AtomicBool::new(false);
static IRONEYE_NORMAL_CAMERA_FOVS: LazyLock<Mutex<Option<IroneyeCameraFovs>>> =
    LazyLock::new(|| Mutex::new(None));
static IRONEYE_NORMAL_GAME_CAMERA: LazyLock<Mutex<Option<IroneyeGameCameraState>>> =
    LazyLock::new(|| Mutex::new(None));
const IRONEYE_VISUAL_FOV_SCALE: f32 = 1.22;
const IRONEYE_VISUAL_CAMERA_PULLBACK: f32 = 2.15;
const IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET: f32 = -0.70;
const IRONEYE_RETICLE_CONVERGENCE_DISTANCE: f32 = 30.0;

#[derive(Clone, Copy)]
struct IroneyeCameraFovs {
    cs: [f32; 4],
    chr: Option<[f32; 4]>,
}

#[derive(Clone, Copy)]
struct IroneyeGameCameraState {
    zoom_target_dist_mult: f32,
    override_lerp_factor: f32,
    zoom_interpolated_progress: f32,
    timed_override_duration: f32,
    zoom_override_lerp_factor: f32,
    zoom_reset_previous_distance: bool,
    override_check_collisions: bool,
}

fn install_ironeye_runtime_tasks_once() {
    INSTALL_IRONEYE_RUNTIME_TASKS.call_once(|| {
        let Ok(task_imp) = CSTaskImp::wait_for_instance(Duration::from_secs(30)) else {
            append_damage_log("[init] ironeye runtime task failed: CSTaskImp unavailable");
            return;
        };

        let pre_behavior = task_imp.run_recurring(
            ironeye_pre_behavior_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PreBehavior,
        );
        let pre_camera = task_imp.run_recurring(
            ironeye_pre_camera_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PreClothSafe,
        );
        let post_camera = task_imp.run_recurring(
            ironeye_post_camera_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DrawParamUpdate,
        );
        let sound_step = task_imp.run_recurring(
            ironeye_precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::SoundStep,
        );
        let post_physics = task_imp.run_recurring(
            ironeye_precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PostPhysics,
        );
        let camera_directed_motion = task_imp.run_recurring(
            camera_directed_motion_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PostPhysics,
        );
        let pre_damage = task_imp.run_recurring(
            ironeye_precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DmgMan_Pre,
        );
        let post_damage = task_imp.run_recurring(
            ironeye_post_damage_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DmgMan_Post,
        );
        let menu_man = task_imp.run_recurring(
            revenant_menu_man_task as fn(&FD4TaskData),
            CSTaskGroupIndex::MenuMan,
        );
        let post_scaleform = task_imp.run_recurring(
            ironeye_post_scaleform_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ScaleformStep,
        );
        let draw_pre = task_imp.run_recurring(
            ironeye_draw_pre_task as fn(&FD4TaskData),
            CSTaskGroupIndex::Draw_Pre,
        );
        std::mem::forget(pre_behavior);
        std::mem::forget(pre_camera);
        std::mem::forget(post_camera);
        std::mem::forget(sound_step);
        std::mem::forget(post_physics);
        std::mem::forget(camera_directed_motion);
        std::mem::forget(pre_damage);
        std::mem::forget(post_damage);
        std::mem::forget(menu_man);
        std::mem::forget(post_scaleform);
        std::mem::forget(draw_pre);
        append_damage_log("[init] ironeye runtime tasks installed");
    });
}

fn ironeye_pre_behavior_task(_: &FD4TaskData) {
    restore_ironeye_precision_before_behavior();
}

fn ironeye_pre_camera_task(_: &FD4TaskData) {
    suppress_ironeye_precision_before_camera();
}

fn ironeye_post_camera_task(_: &FD4TaskData) {
    update_ironeye_precision_view_after_camera();
}

fn ironeye_precision_window_task(_: &FD4TaskData) {
    hide_revenant_native_summon_hud_if_revenant();
    restore_ironeye_precision_for_projectiles();
}

fn camera_directed_motion_task(_: &FD4TaskData) {
    apply_camera_directed_position_delta_if_active();
}

fn ironeye_post_damage_task(_: &FD4TaskData) {
    hide_revenant_native_summon_hud_if_revenant();
    suppress_ironeye_precision_before_menu();
}

fn revenant_menu_man_task(_: &FD4TaskData) {
    hide_revenant_native_summon_hud_if_revenant();
}

fn ironeye_post_scaleform_task(_: &FD4TaskData) {
    hide_revenant_native_summon_hud_if_revenant();
    finish_ironeye_precision_frame_after_scaleform();
}

fn ironeye_draw_pre_task(_: &FD4TaskData) {
    hide_revenant_native_summon_hud_if_revenant();
    restore_ironeye_visual_camera_for_draw();
}

fn restore_ironeye_precision_before_behavior() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        IRONEYE_SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        IRONEYE_SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        IRONEYE_SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    }

    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    let precision_shooting = player.chr_ins.chr_flags1c5.precision_shooting();
    let aiming = manual_attack_aiming || precision_shooting;
    IRONEYE_MANUAL_AIMING.store(aiming, Ordering::Relaxed);
    if !aiming {
        IRONEYE_SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
        IRONEYE_SUPPRESSED_HKS_PRECISION.store(false, Ordering::Relaxed);
        player
            .chr_ins
            .chr_ctrl
            .modifier
            .data
            .hks_flags
            .set_disable_precision_shooting(false);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    }

    restore_normal_game_camera_state();
    if manual_attack_aiming {
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
    player
        .chr_ins
        .chr_ctrl
        .modifier
        .data
        .hks_flags
        .set_disable_precision_shooting(false);
}

fn suppress_ironeye_precision_before_camera() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        return;
    }

    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    let precision_shooting = player.chr_ins.chr_flags1c5.precision_shooting();
    let aiming = manual_attack_aiming || precision_shooting;
    IRONEYE_MANUAL_AIMING.store(aiming, Ordering::Relaxed);
    if precision_shooting {
        IRONEYE_SUPPRESSED_PRECISION.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(false);
    }
}

fn update_ironeye_precision_view_after_camera() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    }

    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    let precision_shooting = player.chr_ins.chr_flags1c5.precision_shooting();
    let aiming =
        manual_attack_aiming || precision_shooting || IRONEYE_SUPPRESSED_PRECISION.load(Ordering::Relaxed);
    IRONEYE_MANUAL_AIMING.store(aiming, Ordering::Relaxed);
    if !aiming {
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    }

    restore_normal_game_camera_state();
    restore_normal_camera_fov();
    restore_normal_game_camera_state();
    if manual_attack_aiming || IRONEYE_SUPPRESSED_PRECISION.load(Ordering::Relaxed) {
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
    IRONEYE_SUPPRESSED_HKS_PRECISION.store(true, Ordering::Relaxed);
    player
        .chr_ins
        .chr_ctrl
        .modifier
        .data
        .hks_flags
        .set_disable_precision_shooting(true);
}

fn restore_ironeye_precision_for_projectiles() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        return;
    }
    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    if manual_attack_aiming || IRONEYE_SUPPRESSED_PRECISION.load(Ordering::Relaxed) {
        IRONEYE_MANUAL_AIMING.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
}

fn suppress_ironeye_precision_before_menu() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        return;
    }
    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    if manual_attack_aiming || player.chr_ins.chr_flags1c5.precision_shooting() {
        IRONEYE_MANUAL_AIMING.store(true, Ordering::Relaxed);
        IRONEYE_SUPPRESSED_PRECISION.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(false);
    }
}

fn finish_ironeye_precision_frame_after_scaleform() {
    if !IRONEYE_SUPPRESSED_PRECISION.swap(false, Ordering::Relaxed) {
        return;
    }
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        IRONEYE_MANUAL_AIMING.store(false, Ordering::Relaxed);
        return;
    }
    if IRONEYE_SUPPRESSED_HKS_PRECISION.swap(false, Ordering::Relaxed) {
        player
            .chr_ins
            .chr_ctrl
            .modifier
            .data
            .hks_flags
            .set_disable_precision_shooting(false);
    }
    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    if manual_attack_aiming {
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
}

fn restore_ironeye_visual_camera_for_draw() {
    if !IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed) {
        return;
    }
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_ref() else {
        return;
    };
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        return;
    }
    restore_normal_camera_fov();
    restore_normal_game_camera_state();
    adjust_ironeye_final_display_camera();
}

#[allow(dead_code)]
fn ironeye_camera_debug_lines() -> Option<[String; 3]> {
    if !IRONEYE_MANUAL_AIMING.load(Ordering::Relaxed) {
        return None;
    }
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    let player = world_chr_man.main_player.as_ref()?;
    let active_effects = player
        .chr_ins
        .special_effect
        .entries()
        .map(|entry| entry.param_id)
        .collect::<Vec<_>>();
    if active_role_from_effects(&active_effects) != Some(Role::Ironeye) {
        return None;
    }
    let camera = unsafe { CSCamera::instance() }.ok()?;
    let chr_cam = world_chr_man.chr_cam.map(|ptr| unsafe { ptr.as_ref() });
    let game_man = unsafe { GameMan::instance() }.ok();
    let manual = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    let precision = player.chr_ins.chr_flags1c5.precision_shooting();
    let hks_disabled = player
        .chr_ins
        .chr_ctrl
        .modifier
        .data
        .hks_flags
        .disable_precision_shooting();
    let cs_line = format!(
        "Ironeye cam flags m:{} p:{} hks:{} mask:{:02X} cs fov {:.3}/{:.3}/{:.3}/{:.3}",
        manual as u8,
        precision as u8,
        hks_disabled as u8,
        camera.camera_mask,
        camera.pers_cam_1.fov,
        camera.pers_cam_2.fov,
        camera.pers_cam_3.fov,
        camera.pers_cam_4.fov,
    );
    let chr_line = if let Some(chr_cam) = chr_cam {
        format!(
            "Ironeye chr type:{:?} fov p/ex/aim/dist {:.3}/{:.3}/{:.3}/{:.3} d p-ex:{:.2} aim-ex:{:.2} cs-ex:{:.2}",
            chr_cam.camera_type,
            chr_cam.pers_cam.fov,
            chr_cam.ex_follow_cam.fov,
            chr_cam.aim_cam.fov,
            chr_cam.dist_view_cam.fov,
            vector4_distance(chr_cam.pers_cam.matrix.3, chr_cam.ex_follow_cam.matrix.3),
            vector4_distance(chr_cam.aim_cam.matrix.3, chr_cam.ex_follow_cam.matrix.3),
            vector4_distance(camera.pers_cam_1.matrix.3, chr_cam.ex_follow_cam.matrix.3),
        )
    } else {
        "Ironeye chr cam unavailable".to_string()
    };
    let game_line = if let Some(game_man) = game_man {
        format!(
            "Ironeye game zoom dist:{:.3} lerp:{:.3} prog:{:.3} dur:{:.3} zlerp:{:.3} reset:{} coll:{}",
            game_man.camera_zoom_target_dist_mult,
            game_man.cam_override_lerp_factor,
            game_man.cam_zoom_interpolated_progress,
            game_man.cam_timed_override_duration,
            game_man.cam_zoom_override_lerp_factor,
            game_man.cam_zoom_reset_previous_distance as u8,
            game_man.cam_override_check_collisions as u8,
        )
    } else {
        "Ironeye game cam unavailable".to_string()
    };
    Some([cs_line, chr_line, game_line])
}

#[allow(dead_code)]
fn vector4_distance(a: fromsoftware_shared::F32Vector4, b: fromsoftware_shared::F32Vector4) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn remember_normal_camera_fov() {
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return;
    };
    let cs = [
        camera.pers_cam_1.fov,
        camera.pers_cam_2.fov,
        camera.pers_cam_3.fov,
        camera.pers_cam_4.fov,
    ];
    let chr = (unsafe { WorldChrMan::instance() })
        .ok()
        .and_then(|world_chr_man| {
            let chr_cam = world_chr_man.chr_cam?;
            let chr_cam = unsafe { chr_cam.as_ref() };
            Some([
                chr_cam.pers_cam.fov,
                chr_cam.ex_follow_cam.fov,
                chr_cam.aim_cam.fov,
                chr_cam.dist_view_cam.fov,
            ])
        });
    let valid_cs = cs
        .iter()
        .all(|fov| fov.is_finite() && *fov > 0.05 && *fov < 3.2);
    let valid_chr = chr
        .map(|fovs| {
            fovs.iter()
                .all(|fov| fov.is_finite() && *fov > 0.05 && *fov < 3.2)
        })
        .unwrap_or(true);
    if valid_cs && valid_chr {
        if let Ok(mut last) = IRONEYE_NORMAL_CAMERA_FOVS.lock() {
            *last = Some(IroneyeCameraFovs { cs, chr });
        }
    }
}

fn restore_normal_camera_fov() {
    let fovs = IRONEYE_NORMAL_CAMERA_FOVS
        .lock()
        .ok()
        .and_then(|last| *last);
    let Some(fovs) = fovs else {
        return;
    };
    if let Ok(camera) = unsafe { CSCamera::instance_mut() } {
        camera.pers_cam_1.fov = fovs.cs[0];
        camera.pers_cam_2.fov = fovs.cs[1];
        camera.pers_cam_3.fov = fovs.cs[2];
        camera.pers_cam_4.fov = fovs.cs[3];
    }
    let Some(chr_fovs) = fovs.chr else {
        return;
    };
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(mut chr_cam) = world_chr_man.chr_cam else {
        return;
    };
    let chr_cam = unsafe { chr_cam.as_mut() };
    let visual_fov = chr_fovs[1].max(0.05);
    chr_cam.pers_cam.fov = visual_fov;
    chr_cam.ex_follow_cam.fov = visual_fov;
    chr_cam.aim_cam.fov = visual_fov;
    chr_cam.dist_view_cam.fov = visual_fov;
}

fn adjust_ironeye_final_display_camera() {
    let Ok(camera) = (unsafe { CSCamera::instance_mut() }) else {
        return;
    };
    let forward = camera.pers_cam_1.forward();
    let right = camera.pers_cam_1.right();
    let len = (forward.0 * forward.0 + forward.1 * forward.1 + forward.2 * forward.2).sqrt();
    let right_len = (right.0 * right.0 + right.1 * right.1 + right.2 * right.2).sqrt();
    if !len.is_finite() || len < 0.001 || !right_len.is_finite() || right_len < 0.001 {
        return;
    }
    let dx = forward.0 / len * IRONEYE_VISUAL_CAMERA_PULLBACK
        + right.0 / right_len * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET;
    let dy = forward.1 / len * IRONEYE_VISUAL_CAMERA_PULLBACK
        + right.1 / right_len * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET;
    let dz = forward.2 / len * IRONEYE_VISUAL_CAMERA_PULLBACK
        + right.2 / right_len * IRONEYE_VISUAL_CAMERA_RIGHT_OFFSET;
    let apply = |cam: &mut eldenring::cs::CSPersCam| {
        cam.fov = (cam.fov * IRONEYE_VISUAL_FOV_SCALE).clamp(0.05, 3.0);
        cam.matrix.3.0 -= dx;
        cam.matrix.3.1 -= dy;
        cam.matrix.3.2 -= dz;
    };
    apply(&mut camera.pers_cam_1);
    apply(&mut camera.pers_cam_2);
    apply(&mut camera.pers_cam_3);
    apply(&mut camera.pers_cam_4);
}

fn ironeye_precision_reticle_screen_pos(viewport: [f32; 2]) -> [f32; 2] {
    let center = [viewport[0] * 0.5, viewport[1] * 0.5];
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
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
    let aim_point = fromsoftware_shared::F32Vector4(
        matrix.3.0 + display_delta[0] + fx * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.1 + display_delta[1] + fy * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.2 + display_delta[2] + fz * IRONEYE_RETICLE_CONVERGENCE_DISTANCE,
        1.0,
    );

    project_lock_on_position(aim_point, cam).unwrap_or(center)
}

fn remember_normal_game_camera_state() {
    let Ok(game_man) = (unsafe { GameMan::instance() }) else {
        return;
    };
    let state = IroneyeGameCameraState {
        zoom_target_dist_mult: game_man.camera_zoom_target_dist_mult,
        override_lerp_factor: game_man.cam_override_lerp_factor,
        zoom_interpolated_progress: game_man.cam_zoom_interpolated_progress,
        timed_override_duration: game_man.cam_timed_override_duration,
        zoom_override_lerp_factor: game_man.cam_zoom_override_lerp_factor,
        zoom_reset_previous_distance: game_man.cam_zoom_reset_previous_distance,
        override_check_collisions: game_man.cam_override_check_collisions,
    };
    if state.zoom_target_dist_mult.is_finite()
        && state.override_lerp_factor.is_finite()
        && state.zoom_interpolated_progress.is_finite()
        && state.timed_override_duration.is_finite()
        && state.zoom_override_lerp_factor.is_finite()
    {
        if let Ok(mut last) = IRONEYE_NORMAL_GAME_CAMERA.lock() {
            *last = Some(state);
        }
    }
}

fn restore_normal_game_camera_state() {
    let state = IRONEYE_NORMAL_GAME_CAMERA
        .lock()
        .ok()
        .and_then(|last| *last);
    let Some(state) = state else {
        return;
    };
    let Ok(game_man) = (unsafe { GameMan::instance_mut() }) else {
        return;
    };
    game_man.camera_zoom_target_dist_mult = state.zoom_target_dist_mult;
    game_man.cam_override_lerp_factor = state.override_lerp_factor;
    game_man.cam_zoom_interpolated_progress = state.zoom_interpolated_progress;
    game_man.cam_timed_override_duration = state.timed_override_duration;
    game_man.cam_zoom_override_lerp_factor = state.zoom_override_lerp_factor;
    game_man.cam_zoom_reset_previous_distance = state.zoom_reset_previous_distance;
    game_man.cam_override_check_collisions = state.override_check_collisions;
}
