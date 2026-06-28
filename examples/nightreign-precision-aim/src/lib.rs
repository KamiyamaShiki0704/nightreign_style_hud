use std::{
    ffi::c_void,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        LazyLock, Mutex, Once,
        atomic::{AtomicBool, AtomicIsize, Ordering},
    },
    thread,
    time::Duration,
};

use eldenring::{
    cs::{CSCamExt, CSCamera, CSTaskGroupIndex, CSTaskImp, CSWindowImp, GameMan, WorldChrMan},
    fd4::FD4TaskData,
    util::system::wait_for_system_init,
};
use fromsoftware_shared::{FromStatic, SharedTaskImpExt, program::Program};
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{
    ImguiRenderLoop, RenderContext,
    hooks::dx12::ImguiDx12Hooks,
    imgui::{Context, ImColor32, TextureId, Ui},
};
use image::io::Reader as ImageReader;
use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
    },
    core::w,
};

static INSTALL_PRECISION_AIM_TASKS: Once = Once::new();
static DLL_MODULE: AtomicIsize = AtomicIsize::new(0);
static PRECISION_AIM_ACTIVE: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_PRECISION: AtomicBool = AtomicBool::new(false);
static SUPPRESSED_HKS_PRECISION: AtomicBool = AtomicBool::new(false);
static NORMAL_CAMERA_FOVS: LazyLock<Mutex<Option<CameraFovs>>> = LazyLock::new(|| Mutex::new(None));
static NORMAL_GAME_CAMERA: LazyLock<Mutex<Option<GameCameraState>>> =
    LazyLock::new(|| Mutex::new(None));

const ACTIVATION_SPEFFECT: i32 = 883100;
const VISUAL_FOV_SCALE: f32 = 1.22;
const VISUAL_CAMERA_PULLBACK: f32 = 2.15;
const VISUAL_CAMERA_RIGHT_OFFSET: f32 = -0.70;
const RETICLE_CONVERGENCE_DISTANCE: f32 = 30.0;
const RETICLE_HALF_SIZE: f32 = 46.0;
const DLL_PROCESS_ATTACH: u32 = 1;
const FULL_HUD_MODULE_NAME: &str = "nightreign_style_hud.dll";
const FULL_HUD_DETECTION_POLLS: usize = 30;
const FULL_HUD_DETECTION_INTERVAL: Duration = Duration::from_millis(100);
const RETICLE_FILES: [[&str; 2]; 3] = [
    ["Precision_Reticle1.png", "Ironeye_Reticle1.png"],
    ["Precision_Reticle2.png", "Ironeye_Reticle2.png"],
    ["Precision_Reticle3.png", "Ironeye_Reticle3.png"],
];
const EMBEDDED_RETICLES: [&[u8]; 3] = [
    include_bytes!("../assets/Ironeye_Reticle1.png").as_slice(),
    include_bytes!("../assets/Ironeye_Reticle2.png").as_slice(),
    include_bytes!("../assets/Ironeye_Reticle3.png").as_slice(),
];

#[unsafe(no_mangle)]
/// # Safety
pub unsafe extern "C" fn DllMain(hmodule: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    DLL_MODULE.store(hmodule.0 as isize, Ordering::Relaxed);
    if reason != DLL_PROCESS_ATTACH {
        return 1;
    }

    let hmodule_raw = hmodule.0 as isize;
    thread::spawn(move || {
        for _ in 0..FULL_HUD_DETECTION_POLLS {
            if full_hud_module_loaded() {
                return;
            }
            thread::sleep(FULL_HUD_DETECTION_INTERVAL);
        }

        let hmodule = HINSTANCE(hmodule_raw as *mut _);
        let _ = debug::initialize::<ImguiDx12Hooks>(
            hmodule,
            DLL_PROCESS_ATTACH,
            || {
                wait_for_system_init(&Program::current(), Duration::MAX)
                    .expect("Timeout waiting for system init");
            },
            PrecisionAimHud::default(),
        );
    });

    1
}

fn full_hud_module_loaded() -> bool {
    let _ = FULL_HUD_MODULE_NAME;
    unsafe { GetModuleHandleW(w!("nightreign_style_hud.dll")).is_ok() }
}

#[derive(Clone, Copy)]
struct CameraFovs {
    cs: [f32; 4],
    chr: Option<[f32; 4]>,
}

#[derive(Clone, Copy)]
struct GameCameraState {
    zoom_target_dist_mult: f32,
    override_lerp_factor: f32,
    zoom_interpolated_progress: f32,
    timed_override_duration: f32,
    zoom_override_lerp_factor: f32,
    zoom_reset_previous_distance: bool,
    override_check_collisions: bool,
}

#[derive(Default)]
struct PrecisionAimHud {
    scale: f32,
    reticles: [Option<TextureId>; 3],
}

impl ImguiRenderLoop for PrecisionAimHud {
    fn initialize(&mut self, ctx: &mut Context, render_context: &mut dyn RenderContext) {
        ctx.io_mut().config_flags |= hudhook::imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
        self.reticles = load_reticles(render_context);
        install_precision_aim_tasks_once();
    }

    fn render(&mut self, ui: &mut Ui) {
        self.update_scale();
        if !PRECISION_AIM_ACTIVE.load(Ordering::Relaxed) {
            return;
        }
        draw_precision_reticle(ui, self.reticles, self.scale);
    }
}

impl PrecisionAimHud {
    fn update_scale(&mut self) {
        self.scale = unsafe { CSWindowImp::instance() }
            .map(|window| (window.screen_width as f32 / 1920.0).clamp(0.65, 1.5))
            .unwrap_or(1.0);
    }
}

fn install_precision_aim_tasks_once() {
    INSTALL_PRECISION_AIM_TASKS.call_once(|| {
        let Ok(task_imp) = CSTaskImp::wait_for_instance(Duration::from_secs(30)) else {
            return;
        };

        let pre_behavior = task_imp.run_recurring(
            pre_behavior_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PreBehavior,
        );
        let pre_camera = task_imp.run_recurring(
            pre_camera_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PreClothSafe,
        );
        let post_camera = task_imp.run_recurring(
            post_camera_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DrawParamUpdate,
        );
        let sound_step = task_imp.run_recurring(
            precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::SoundStep,
        );
        let post_physics = task_imp.run_recurring(
            precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ChrIns_PostPhysics,
        );
        let pre_damage = task_imp.run_recurring(
            precision_window_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DmgMan_Pre,
        );
        let post_damage = task_imp.run_recurring(
            post_damage_task as fn(&FD4TaskData),
            CSTaskGroupIndex::DmgMan_Post,
        );
        let post_scaleform = task_imp.run_recurring(
            post_scaleform_task as fn(&FD4TaskData),
            CSTaskGroupIndex::ScaleformStep,
        );
        let draw_pre = task_imp.run_recurring(
            draw_pre_task as fn(&FD4TaskData),
            CSTaskGroupIndex::Draw_Pre,
        );
        std::mem::forget(pre_behavior);
        std::mem::forget(pre_camera);
        std::mem::forget(post_camera);
        std::mem::forget(sound_step);
        std::mem::forget(post_physics);
        std::mem::forget(pre_damage);
        std::mem::forget(post_damage);
        std::mem::forget(post_scaleform);
        std::mem::forget(draw_pre);
    });
}

fn pre_behavior_task(_: &FD4TaskData) {
    restore_precision_before_behavior();
}

fn pre_camera_task(_: &FD4TaskData) {
    suppress_precision_before_camera();
}

fn post_camera_task(_: &FD4TaskData) {
    update_precision_view_after_camera();
}

fn precision_window_task(_: &FD4TaskData) {
    restore_precision_for_projectiles();
}

fn post_damage_task(_: &FD4TaskData) {
    suppress_precision_before_menu();
}

fn post_scaleform_task(_: &FD4TaskData) {
    finish_precision_frame_after_scaleform();
}

fn draw_pre_task(_: &FD4TaskData) {
    restore_visual_camera_for_draw();
}

fn restore_precision_before_behavior() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        reset_precision_state();
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        reset_precision_state();
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    if !player_has_activation_speffect(player) {
        reset_precision_state();
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

    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    let precision_shooting = player.chr_ins.chr_flags1c5.precision_shooting();
    let aiming = manual_attack_aiming || precision_shooting;
    PRECISION_AIM_ACTIVE.store(aiming, Ordering::Relaxed);
    if !aiming {
        SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
        SUPPRESSED_HKS_PRECISION.store(false, Ordering::Relaxed);
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

fn suppress_precision_before_camera() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    if !player_has_activation_speffect(player) {
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
    PRECISION_AIM_ACTIVE.store(aiming, Ordering::Relaxed);
    if precision_shooting {
        SUPPRESSED_PRECISION.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(false);
    }
}

fn update_precision_view_after_camera() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        PRECISION_AIM_ACTIVE.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        PRECISION_AIM_ACTIVE.store(false, Ordering::Relaxed);
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    };
    if !player_has_activation_speffect(player) {
        PRECISION_AIM_ACTIVE.store(false, Ordering::Relaxed);
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
        manual_attack_aiming || precision_shooting || SUPPRESSED_PRECISION.load(Ordering::Relaxed);
    PRECISION_AIM_ACTIVE.store(aiming, Ordering::Relaxed);
    if !aiming {
        remember_normal_camera_fov();
        remember_normal_game_camera_state();
        return;
    }

    restore_normal_game_camera_state();
    restore_normal_camera_fov();
    restore_normal_game_camera_state();
    if manual_attack_aiming || SUPPRESSED_PRECISION.load(Ordering::Relaxed) {
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
    SUPPRESSED_HKS_PRECISION.store(true, Ordering::Relaxed);
    player
        .chr_ins
        .chr_ctrl
        .modifier
        .data
        .hks_flags
        .set_disable_precision_shooting(true);
}

fn restore_precision_for_projectiles() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    if !player_has_activation_speffect(player) {
        return;
    }
    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    if manual_attack_aiming || SUPPRESSED_PRECISION.load(Ordering::Relaxed) {
        PRECISION_AIM_ACTIVE.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(true);
    }
}

fn suppress_precision_before_menu() {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    if !player_has_activation_speffect(player) {
        return;
    }
    let manual_attack_aiming = player
        .chr_ins
        .modules
        .action_flag
        .action_modifiers_flags
        .manual_attack_aiming();
    if manual_attack_aiming || player.chr_ins.chr_flags1c5.precision_shooting() {
        PRECISION_AIM_ACTIVE.store(true, Ordering::Relaxed);
        SUPPRESSED_PRECISION.store(true, Ordering::Relaxed);
        player.chr_ins.chr_flags1c5.set_precision_shooting(false);
    }
}

fn finish_precision_frame_after_scaleform() {
    if !SUPPRESSED_PRECISION.swap(false, Ordering::Relaxed) {
        return;
    }
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance_mut() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_mut() else {
        return;
    };
    if !player_has_activation_speffect(player) {
        PRECISION_AIM_ACTIVE.store(false, Ordering::Relaxed);
        return;
    }
    if SUPPRESSED_HKS_PRECISION.swap(false, Ordering::Relaxed) {
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

fn restore_visual_camera_for_draw() {
    if !PRECISION_AIM_ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return;
    };
    let Some(player) = world_chr_man.main_player.as_ref() else {
        return;
    };
    if !player_has_activation_speffect(player) {
        return;
    }
    restore_normal_camera_fov();
    restore_normal_game_camera_state();
    adjust_final_display_camera();
}

fn reset_precision_state() {
    PRECISION_AIM_ACTIVE.store(false, Ordering::Relaxed);
    SUPPRESSED_PRECISION.store(false, Ordering::Relaxed);
}

fn player_has_activation_speffect(player: &eldenring::cs::PlayerIns) -> bool {
    player
        .chr_ins
        .special_effect
        .entries()
        .any(|entry| entry.param_id == ACTIVATION_SPEFFECT)
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
        if let Ok(mut last) = NORMAL_CAMERA_FOVS.lock() {
            *last = Some(CameraFovs { cs, chr });
        }
    }
}

fn restore_normal_camera_fov() {
    let fovs = NORMAL_CAMERA_FOVS.lock().ok().and_then(|last| *last);
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

fn adjust_final_display_camera() {
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
    let dx =
        forward.0 / len * VISUAL_CAMERA_PULLBACK + right.0 / right_len * VISUAL_CAMERA_RIGHT_OFFSET;
    let dy =
        forward.1 / len * VISUAL_CAMERA_PULLBACK + right.1 / right_len * VISUAL_CAMERA_RIGHT_OFFSET;
    let dz =
        forward.2 / len * VISUAL_CAMERA_PULLBACK + right.2 / right_len * VISUAL_CAMERA_RIGHT_OFFSET;
    let apply = |cam: &mut eldenring::cs::CSPersCam| {
        cam.fov = (cam.fov * VISUAL_FOV_SCALE).clamp(0.05, 3.0);
        cam.matrix.3.0 -= dx;
        cam.matrix.3.1 -= dy;
        cam.matrix.3.2 -= dz;
    };
    apply(&mut camera.pers_cam_1);
    apply(&mut camera.pers_cam_2);
    apply(&mut camera.pers_cam_3);
    apply(&mut camera.pers_cam_4);
}

fn reticle_screen_pos(viewport: [f32; 2]) -> [f32; 2] {
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
        fx * VISUAL_CAMERA_PULLBACK + rx * VISUAL_CAMERA_RIGHT_OFFSET,
        fy * VISUAL_CAMERA_PULLBACK + ry * VISUAL_CAMERA_RIGHT_OFFSET,
        fz * VISUAL_CAMERA_PULLBACK + rz * VISUAL_CAMERA_RIGHT_OFFSET,
    ];
    let matrix = cam.matrix;
    let aim_point = fromsoftware_shared::F32Vector4(
        matrix.3.0 + display_delta[0] + fx * RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.1 + display_delta[1] + fy * RETICLE_CONVERGENCE_DISTANCE,
        matrix.3.2 + display_delta[2] + fz * RETICLE_CONVERGENCE_DISTANCE,
        1.0,
    );

    project_position(aim_point, cam).unwrap_or(center)
}

fn remember_normal_game_camera_state() {
    let Ok(game_man) = (unsafe { GameMan::instance() }) else {
        return;
    };
    let state = GameCameraState {
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
        if let Ok(mut last) = NORMAL_GAME_CAMERA.lock() {
            *last = Some(state);
        }
    }
}

fn restore_normal_game_camera_state() {
    let state = NORMAL_GAME_CAMERA.lock().ok().and_then(|last| *last);
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

fn draw_precision_reticle(ui: &Ui, reticles: [Option<TextureId>; 3], scale: f32) {
    let viewport = ui.io().display_size;
    let center = reticle_screen_pos(viewport);
    let mut drew_reticle = false;
    for texture_id in reticles.into_iter().flatten() {
        draw_centered_texture(
            ui,
            center,
            RETICLE_HALF_SIZE * scale,
            texture_id,
            ImColor32::from_rgba(255, 255, 255, 235),
        );
        drew_reticle = true;
    }
    if !drew_reticle {
        draw_fallback_reticle(ui, center, scale);
    }
}

fn draw_centered_texture(
    ui: &Ui,
    center: [f32; 2],
    half: f32,
    texture_id: TextureId,
    color: ImColor32,
) {
    ui.get_foreground_draw_list()
        .add_image(
            texture_id,
            [center[0] - half, center[1] - half],
            [center[0] + half, center[1] + half],
        )
        .col(color)
        .build();
}

fn draw_fallback_reticle(ui: &Ui, center: [f32; 2], scale: f32) {
    let draw = ui.get_foreground_draw_list();
    let half = RETICLE_HALF_SIZE * 0.42 * scale;
    let color = ImColor32::from_rgba(244, 247, 255, 220);
    draw.add_line(
        [center[0] - half, center[1]],
        [center[0] + half, center[1]],
        color,
    )
    .thickness(1.5 * scale)
    .build();
    draw.add_line(
        [center[0], center[1] - half],
        [center[0], center[1] + half],
        color,
    )
    .thickness(1.5 * scale)
    .build();
    draw.add_circle(center, half * 0.36, color)
        .num_segments(48)
        .thickness(1.4 * scale)
        .build();
}

fn load_reticles(render_context: &mut dyn RenderContext) -> [Option<TextureId>; 3] {
    [
        load_customizable_texture(render_context, &RETICLE_FILES[0], EMBEDDED_RETICLES[0]),
        load_customizable_texture(render_context, &RETICLE_FILES[1], EMBEDDED_RETICLES[1]),
        load_customizable_texture(render_context, &RETICLE_FILES[2], EMBEDDED_RETICLES[2]),
    ]
}

fn load_customizable_texture(
    render_context: &mut dyn RenderContext,
    file_names: &[&str; 2],
    embedded: &[u8],
) -> Option<TextureId> {
    for dir in external_asset_dirs() {
        for file_name in file_names {
            let path = dir.join(file_name);
            if let Ok(bytes) = std::fs::read(&path) {
                if let Some(texture) = load_texture_bytes(render_context, &bytes) {
                    return Some(texture);
                }
            }
        }
    }
    load_texture_bytes(render_context, embedded)
}

fn external_asset_dirs() -> Vec<PathBuf> {
    dll_module_dir()
        .map(|dir| vec![dir.join("assets")])
        .unwrap_or_default()
}

fn dll_module_dir() -> Option<PathBuf> {
    let raw_module = DLL_MODULE.load(Ordering::Relaxed);
    if raw_module == 0 {
        return None;
    }
    let mut buffer = [0u16; 32768];
    let len = unsafe { GetModuleFileNameW(Some(HMODULE(raw_module as *mut c_void)), &mut buffer) }
        as usize;
    if len == 0 || len >= buffer.len() {
        return None;
    }
    let path = PathBuf::from(String::from_utf16_lossy(&buffer[..len]));
    path.parent().map(Path::to_path_buf)
}

fn load_texture_bytes(render_context: &mut dyn RenderContext, bytes: &[u8]) -> Option<TextureId> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .into_rgba8();
    render_context
        .load_texture(image.as_raw(), image.width(), image.height())
        .ok()
}

fn project_position(
    position: fromsoftware_shared::F32Vector4,
    camera: &eldenring::cs::CSPersCam,
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

    let axes = (
        [matrix.0.0, matrix.0.1, matrix.0.2],
        [matrix.1.0, matrix.1.1, matrix.1.2],
        [matrix.2.0, matrix.2.1, matrix.2.2],
    );
    project_with_camera_axes(rel, axes, width, height, aspect, tan_half_fov)
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
