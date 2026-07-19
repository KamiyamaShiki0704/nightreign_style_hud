use std::{ffi::c_void, time::Duration};

use eldenring::{
    cs::{
        CSCamExt, CSCamera, CSHavokMan, CSWindowImp, GameMan, PlayerIns, WorldChrMan,
    },
    position::{HavokPosition, PositionDelta},
    util::system::wait_for_system_init,
};
use fromsoftware_shared::{F32Vector4, FromStatic, program::Program};
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{
    ImguiRenderLoop,
    hooks::dx12::ImguiDx12Hooks,
    imgui::{Context, ImColor32, Ui, sys},
};

const DEBUG_MAX_DISTANCE: f32 = 60.0;
const DEBUG_RAY_FILTERS: [u32; 9] = [0, 1, 2, 4, 8, 16, 32, 64, 128];
const DEBUG_RAY_SAMPLE_DISTANCES: [f32; 7] = [2.0, 4.0, 8.0, 16.0, 24.0, 40.0, 60.0];
const DEBUG_TEXT_SIZE: f32 = 23.0;
const DEBUG_TEXT_STEP: f32 = 28.0;
const DEBUG_RAY_NEAR_SCALE_DISTANCE: f32 = 2.0;
const DEBUG_RAY_NEAR_MARKER_SIZE: f32 = 18.0;
const DEBUG_RAY_FAR_MARKER_SIZE: f32 = 4.0;
const DEBUG_RAY_NEAR_LINE_THICKNESS: f32 = 4.0;
const DEBUG_RAY_FAR_LINE_THICKNESS: f32 = 1.0;

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
        CameraDebugHud,
    )
}

struct CameraDebugHud;

impl ImguiRenderLoop for CameraDebugHud {
    fn initialize(
        &mut self,
        ctx: &mut Context,
        _render_context: &mut dyn hudhook::RenderContext,
    ) {
        ctx.io_mut().config_flags |= hudhook::imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
    }

    fn render(&mut self, ui: &mut Ui) {
        let snapshot = camera_debug_snapshot();
        draw_camera_debug_overlay(ui, snapshot);
    }
}

#[derive(Clone, Copy)]
struct CameraDebugSnapshot {
    valid: bool,
    screen: [f32; 2],
    camera_mask: u32,
    camera_pos: [f32; 3],
    camera_forward: [f32; 3],
    camera_right: [f32; 3],
    camera_up: [f32; 3],
    camera_fov: f32,
    camera_aspect: f32,
    camera_near: f32,
    camera_far: f32,
    camera_yaw_deg: f32,
    camera_pitch_deg: f32,
    player_pos: [f32; 3],
    player_hit_height: f32,
    ray_origin: [f32; 3],
    ray_start_screen: Option<[f32; 2]>,
    ray_end_screen: Option<[f32; 2]>,
    ray_hit: Option<RayHitDebug>,
    ray_samples: [Option<[f32; 2]>; DEBUG_RAY_SAMPLE_DISTANCES.len()],
    camera_to_player: CameraRelativeDebug,
    game_camera: GameCameraDebug,
}

#[derive(Clone, Copy)]
struct RayHitDebug {
    filter: u32,
    distance: f32,
    world: [f32; 3],
    screen: Option<[f32; 2]>,
}

#[derive(Clone, Copy)]
struct CameraRelativeDebug {
    distance: f32,
    depth: f32,
    side: f32,
    vertical: f32,
    height_delta: f32,
}

#[derive(Clone, Copy)]
struct GameCameraDebug {
    normal_param: i32,
    locked_param: i32,
    talk_param: i32,
    lock_on_param: i32,
    chr_lock_on_param: i32,
    follow_dummy_poly: i32,
    zoom_target_dist_mult: f32,
    override_lerp_factor: f32,
    zoom_progress: f32,
    timed_override_duration: f32,
    zoom_override_lerp_factor: f32,
    force_vertical_deg: f32,
    force_horizontal_deg: f32,
    force_method: u32,
    force_progress: f32,
    force_vertical_enabled: bool,
    force_horizontal_enabled: bool,
    force_first_execution: bool,
}

impl Default for CameraDebugSnapshot {
    fn default() -> Self {
        Self {
            valid: false,
            screen: [0.0, 0.0],
            camera_mask: 0,
            camera_pos: [0.0, 0.0, 0.0],
            camera_forward: [0.0, 0.0, 0.0],
            camera_right: [0.0, 0.0, 0.0],
            camera_up: [0.0, 0.0, 0.0],
            camera_fov: 0.0,
            camera_aspect: 0.0,
            camera_near: 0.0,
            camera_far: 0.0,
            camera_yaw_deg: 0.0,
            camera_pitch_deg: 0.0,
            player_pos: [0.0, 0.0, 0.0],
            player_hit_height: 0.0,
            ray_origin: [0.0, 0.0, 0.0],
            ray_start_screen: None,
            ray_end_screen: None,
            ray_hit: None,
            ray_samples: [None; DEBUG_RAY_SAMPLE_DISTANCES.len()],
            camera_to_player: CameraRelativeDebug::default(),
            game_camera: GameCameraDebug::default(),
        }
    }
}

impl Default for CameraRelativeDebug {
    fn default() -> Self {
        Self {
            distance: 0.0,
            depth: 0.0,
            side: 0.0,
            vertical: 0.0,
            height_delta: 0.0,
        }
    }
}

impl Default for GameCameraDebug {
    fn default() -> Self {
        Self {
            normal_param: 0,
            locked_param: 0,
            talk_param: 0,
            lock_on_param: 0,
            chr_lock_on_param: 0,
            follow_dummy_poly: 0,
            zoom_target_dist_mult: 0.0,
            override_lerp_factor: 0.0,
            zoom_progress: 0.0,
            timed_override_duration: 0.0,
            zoom_override_lerp_factor: 0.0,
            force_vertical_deg: 0.0,
            force_horizontal_deg: 0.0,
            force_method: 0,
            force_progress: 0.0,
            force_vertical_enabled: false,
            force_horizontal_enabled: false,
            force_first_execution: false,
        }
    }
}

fn camera_debug_snapshot() -> CameraDebugSnapshot {
    let Ok(window) = (unsafe { CSWindowImp::instance() }) else {
        return CameraDebugSnapshot::default();
    };
    let Ok(camera) = (unsafe { CSCamera::instance() }) else {
        return CameraDebugSnapshot::default();
    };
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else {
        return CameraDebugSnapshot::default();
    };
    let Some(player) = world_chr_man.main_player.as_ref() else {
        return CameraDebugSnapshot::default();
    };

    let width = window.screen_width as f32;
    let height = window.screen_height as f32;
    if width <= 0.0 || height <= 0.0 {
        return CameraDebugSnapshot::default();
    }

    let cam = camera.pers_cam_1.as_ref();
    let camera_pos = havok_to_array(cam.position());
    let forward = normalize_delta(cam.forward()).unwrap_or([0.0, 0.0, 1.0]);
    let right = normalize_delta(cam.right()).unwrap_or([1.0, 0.0, 0.0]);
    let up = normalize_delta(cam.up()).unwrap_or([0.0, 1.0, 0.0]);
    let player_physics = player.chr_ins.modules.as_ref().physics.as_ref();
    let player_pos = havok_to_array(player_physics.position);
    let camera_position = cam.position();
    let camera_to_player = [
        player_physics.position.0 - camera_position.0,
        player_physics.position.1 - camera_position.1,
        player_physics.position.2 - camera_position.2,
    ];
    let camera_to_player_distance = length3(camera_to_player);
    let camera_relative = CameraRelativeDebug {
        distance: camera_to_player_distance,
        depth: dot3(camera_to_player, forward),
        side: dot3(camera_to_player, right),
        vertical: dot3(camera_to_player, up),
        height_delta: player_physics.position.1 - camera_position.1,
    };
    let ray_origin = HavokPosition(
        player_physics.position.0,
        player_physics.position.1 + (player_physics.hit_height * 0.62).clamp(0.8, 2.2),
        player_physics.position.2,
        0.0,
    );
    let ray_hit = center_ray_hit(player, ray_origin, forward);
    let ray_start_screen = project_position(
        F32Vector4(ray_origin.0, ray_origin.1, ray_origin.2, 1.0),
        cam,
    );
    let ray_end_world = ray_hit.map(|hit| hit.world).unwrap_or([
        ray_origin.0 + forward[0] * DEBUG_MAX_DISTANCE,
        ray_origin.1 + forward[1] * DEBUG_MAX_DISTANCE,
        ray_origin.2 + forward[2] * DEBUG_MAX_DISTANCE,
    ]);
    let ray_end_screen = ray_hit.and_then(|hit| hit.screen).or_else(|| {
        project_position(
            F32Vector4(ray_end_world[0], ray_end_world[1], ray_end_world[2], 1.0),
            cam,
        )
    });
    let game_camera = unsafe { GameMan::instance() }
        .ok()
        .map(game_camera_debug)
        .unwrap_or_default();

    CameraDebugSnapshot {
        valid: true,
        screen: [width, height],
        camera_mask: camera.camera_mask,
        camera_pos,
        camera_forward: forward,
        camera_right: right,
        camera_up: up,
        camera_fov: cam.fov,
        camera_aspect: cam.aspect_ratio,
        camera_near: cam.near_plane,
        camera_far: cam.far_plane,
        camera_yaw_deg: forward[0].atan2(forward[2]).to_degrees(),
        camera_pitch_deg: forward[1].clamp(-1.0, 1.0).asin().to_degrees(),
        player_pos,
        player_hit_height: player_physics.hit_height,
        ray_origin: havok_to_array(ray_origin),
        ray_start_screen,
        ray_end_screen,
        ray_hit,
        ray_samples: forward_sample_points(cam, ray_origin, forward),
        camera_to_player: camera_relative,
        game_camera,
    }
}

fn game_camera_debug(game_man: &GameMan) -> GameCameraDebug {
    GameCameraDebug {
        normal_param: game_man.normal_camera_param_id,
        locked_param: game_man.locked_camera_param_id,
        talk_param: game_man.talk_esd_camera_param_id,
        lock_on_param: game_man.lock_on_camera_param_id,
        chr_lock_on_param: game_man.camera_chr_lock_on_param_id,
        follow_dummy_poly: game_man.camera_follow_dummy_poly_id,
        zoom_target_dist_mult: game_man.camera_zoom_target_dist_mult,
        override_lerp_factor: game_man.cam_override_lerp_factor,
        zoom_progress: game_man.cam_zoom_interpolated_progress,
        timed_override_duration: game_man.cam_timed_override_duration,
        zoom_override_lerp_factor: game_man.cam_zoom_override_lerp_factor,
        force_vertical_deg: game_man.force_cam_vertical_angle_rad.to_degrees(),
        force_horizontal_deg: game_man.force_cam_horizontal_angle_rad.to_degrees(),
        force_method: game_man.force_cam_rotation_method as u32,
        force_progress: game_man.force_cam_interpolation_progress,
        force_vertical_enabled: game_man.force_cam_vertical_enabled,
        force_horizontal_enabled: game_man.force_cam_horizontal_enabled,
        force_first_execution: game_man.force_cam_first_execution,
    }
}

fn center_ray_hit(player: &PlayerIns, origin: HavokPosition, forward: [f32; 3]) -> Option<RayHitDebug> {
    let Ok(havok) = (unsafe { CSHavokMan::instance() }) else {
        return None;
    };
    let delta = PositionDelta(
        forward[0] * DEBUG_MAX_DISTANCE,
        forward[1] * DEBUG_MAX_DISTANCE,
        forward[2] * DEBUG_MAX_DISTANCE,
    );
    let mut best: Option<RayHitDebug> = None;
    for filter in DEBUG_RAY_FILTERS {
        let Some(hit) = havok
            .phys_world
            .as_ref()
            .cast_ray(filter, &origin, delta, player)
        else {
            continue;
        };
        let distance = distance_havok(origin, hit);
        if !distance.is_finite() || distance <= 0.0 || distance > DEBUG_MAX_DISTANCE {
            continue;
        }
        if best.is_some_and(|best| distance >= best.distance) {
            continue;
        }
        let screen = unsafe { CSCamera::instance() }
            .ok()
            .and_then(|camera| {
                project_position(
                    F32Vector4(hit.0, hit.1, hit.2, 1.0),
                    camera.pers_cam_1.as_ref(),
                )
            });
        best = Some(RayHitDebug {
            filter,
            distance,
            world: havok_to_array(hit),
            screen,
        });
    }
    best
}

fn forward_sample_points(
    cam: &eldenring::cs::CSPersCam,
    origin: HavokPosition,
    forward: [f32; 3],
) -> [Option<[f32; 2]>; DEBUG_RAY_SAMPLE_DISTANCES.len()] {
    DEBUG_RAY_SAMPLE_DISTANCES.map(|distance| {
        project_position(
            F32Vector4(
                origin.0 + forward[0] * distance,
                origin.1 + forward[1] * distance,
                origin.2 + forward[2] * distance,
                1.0,
            ),
            cam,
        )
    })
}

fn project_position(position: F32Vector4, camera: &eldenring::cs::CSPersCam) -> Option<[f32; 2]> {
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

fn draw_camera_debug_overlay(ui: &Ui, snapshot: CameraDebugSnapshot) {
    let draw = ui.get_foreground_draw_list();
    if !snapshot.valid {
        draw_text_shadowed(
            [28.0, 28.0],
            DEBUG_TEXT_SIZE,
            ImColor32::from_rgba(255, 90, 80, 235),
            "CameraDebug: waiting for game state",
        );
        return;
    }

    let center = [snapshot.screen[0] * 0.5, snapshot.screen[1] * 0.5];
    let white = ImColor32::from_rgba(235, 245, 255, 210);
    let cyan = ImColor32::from_rgba(85, 225, 255, 220);
    let yellow = ImColor32::from_rgba(255, 210, 70, 235);
    let orange = ImColor32::from_rgba(255, 154, 70, 235);

    draw.add_line([center[0] - 26.0, center[1]], [center[0] + 26.0, center[1]], white)
        .thickness(2.2)
        .build();
    draw.add_line([center[0], center[1] - 26.0], [center[0], center[1] + 26.0], white)
        .thickness(2.2)
        .build();
    draw.add_circle(center, 7.0, white)
        .num_segments(32)
        .thickness(2.0)
        .build();

    if let Some(end) = snapshot.ray_end_screen {
        let end_distance = snapshot
            .ray_hit
            .map(|hit| hit.distance)
            .unwrap_or(DEBUG_MAX_DISTANCE);
        draw.add_line(center, end, orange)
            .thickness(ray_visual_line_thickness(end_distance))
            .build();
        draw_marker(end, orange, ray_visual_marker_size(end_distance));
    }
    if let (Some(start), Some(end)) = (snapshot.ray_start_screen, snapshot.ray_end_screen) {
        draw.add_line(start, end, cyan)
            .thickness(ray_visual_line_thickness(DEBUG_MAX_DISTANCE) * 0.75)
            .build();
        draw_marker(start, cyan, ray_visual_marker_size(0.0));
    }

    let mut previous = snapshot.ray_start_screen;
    let mut previous_distance = 0.0;
    for (index, sample) in snapshot.ray_samples.into_iter().enumerate() {
        if let Some(pos) = sample {
            let distance = DEBUG_RAY_SAMPLE_DISTANCES[index];
            if let Some(prev) = previous {
                let segment_distance = (previous_distance + distance) * 0.5;
                draw.add_line(prev, pos, cyan)
                    .thickness(ray_visual_line_thickness(segment_distance))
                    .build();
            }
            previous = Some(pos);
            previous_distance = distance;
            draw.add_circle(pos, ray_visual_marker_size(distance), cyan)
                .num_segments(24)
                .filled(true)
                .build();
        }
    }
    if let Some(hit) = snapshot.ray_hit.and_then(|hit| hit.screen.map(|screen| (hit, screen))) {
        draw_marker(hit.1, yellow, ray_visual_marker_size(hit.0.distance) * 1.35);
    }

    let ray_text = snapshot
        .ray_hit
        .map(|hit| {
            format!(
                "ray hit filter:{} dist:{:.2} world:{:.2}/{:.2}/{:.2} screen:{}",
                hit.filter,
                hit.distance,
                hit.world[0],
                hit.world[1],
                hit.world[2],
                hit.screen
                    .map(|p| format!("{:.0}/{:.0}", p[0], p[1]))
                    .unwrap_or_else(|| "off".to_string())
            )
        })
        .unwrap_or_else(|| "ray hit none".to_string());

    let lines = [
        format!(
            "cam pos:{:.2}/{:.2}/{:.2}  mask:{:#04X}",
            snapshot.camera_pos[0],
            snapshot.camera_pos[1],
            snapshot.camera_pos[2],
            snapshot.camera_mask,
        ),
        format!(
            "fwd:{:.3}/{:.3}/{:.3}  yaw:{:.1} pitch:{:.1}",
            snapshot.camera_forward[0],
            snapshot.camera_forward[1],
            snapshot.camera_forward[2],
            snapshot.camera_yaw_deg,
            snapshot.camera_pitch_deg
        ),
        format!(
            "right:{:.3}/{:.3}/{:.3}  up:{:.3}/{:.3}/{:.3}",
            snapshot.camera_right[0],
            snapshot.camera_right[1],
            snapshot.camera_right[2],
            snapshot.camera_up[0],
            snapshot.camera_up[1],
            snapshot.camera_up[2]
        ),
        format!(
            "fov:{:.3} aspect:{:.3} near/far:{:.2}/{:.1} screen:{:.0}x{:.0}",
            snapshot.camera_fov,
            snapshot.camera_aspect,
            snapshot.camera_near,
            snapshot.camera_far,
            snapshot.screen[0],
            snapshot.screen[1],
        ),
        format!(
            "player pos:{:.2}/{:.2}/{:.2} hit_h:{:.2} ray_origin:{:.2}/{:.2}/{:.2}",
            snapshot.player_pos[0],
            snapshot.player_pos[1],
            snapshot.player_pos[2],
            snapshot.player_hit_height,
            snapshot.ray_origin[0],
            snapshot.ray_origin[1],
            snapshot.ray_origin[2],
        ),
        format!(
            "cam->player dist:{:.2} depth:{:.2} side:{:.2} up:{:.2} y_delta:{:.2}",
            snapshot.camera_to_player.distance,
            snapshot.camera_to_player.depth,
            snapshot.camera_to_player.side,
            snapshot.camera_to_player.vertical,
            snapshot.camera_to_player.height_delta,
        ),
        ray_text,
        format!(
            "cam param normal:{} locked:{} talk:{} lock_on:{} chr_lock:{} follow_dmy:{}",
            snapshot.game_camera.normal_param,
            snapshot.game_camera.locked_param,
            snapshot.game_camera.talk_param,
            snapshot.game_camera.lock_on_param,
            snapshot.game_camera.chr_lock_on_param,
            snapshot.game_camera.follow_dummy_poly,
        ),
        format!(
            "zoom mult:{:.2} lerp:{:.2} progress:{:.2} dur:{:.2} zoom_lerp:{:.2}",
            snapshot.game_camera.zoom_target_dist_mult,
            snapshot.game_camera.override_lerp_factor,
            snapshot.game_camera.zoom_progress,
            snapshot.game_camera.timed_override_duration,
            snapshot.game_camera.zoom_override_lerp_factor,
        ),
        format!(
            "force cam v/h:{:.1}/{:.1} method:{} progress:{:.2} en v/h:{}/{} first:{}",
            snapshot.game_camera.force_vertical_deg,
            snapshot.game_camera.force_horizontal_deg,
            snapshot.game_camera.force_method,
            snapshot.game_camera.force_progress,
            snapshot.game_camera.force_vertical_enabled as u8,
            snapshot.game_camera.force_horizontal_enabled as u8,
            snapshot.game_camera.force_first_execution as u8,
        ),
    ];
    for (index, line) in lines.iter().enumerate() {
        draw_text_shadowed(
            [28.0, 28.0 + index as f32 * DEBUG_TEXT_STEP],
            DEBUG_TEXT_SIZE,
            white,
            &line,
        );
    }
}

fn ray_visual_t(distance: f32) -> f32 {
    ((distance - DEBUG_RAY_NEAR_SCALE_DISTANCE) / (DEBUG_MAX_DISTANCE - DEBUG_RAY_NEAR_SCALE_DISTANCE))
        .clamp(0.0, 1.0)
}

fn ray_visual_marker_size(distance: f32) -> f32 {
    lerp(
        DEBUG_RAY_NEAR_MARKER_SIZE,
        DEBUG_RAY_FAR_MARKER_SIZE,
        ray_visual_t(distance),
    )
}

fn ray_visual_line_thickness(distance: f32) -> f32 {
    lerp(
        DEBUG_RAY_NEAR_LINE_THICKNESS,
        DEBUG_RAY_FAR_LINE_THICKNESS,
        ray_visual_t(distance),
    )
}

fn draw_marker(pos: [f32; 2], color: ImColor32, size: f32) {
    unsafe {
        let draw = sys::igGetForegroundDrawList();
        if draw.is_null() {
            return;
        }
        let col: u32 = color.into();
        sys::ImDrawList_AddCircle(
            draw,
            sys::ImVec2 { x: pos[0], y: pos[1] },
            size,
            col,
            48,
            2.0,
        );
        sys::ImDrawList_AddLine(
            draw,
            sys::ImVec2 {
                x: pos[0] - size,
                y: pos[1],
            },
            sys::ImVec2 {
                x: pos[0] + size,
                y: pos[1],
            },
            col,
            2.0,
        );
        sys::ImDrawList_AddLine(
            draw,
            sys::ImVec2 {
                x: pos[0],
                y: pos[1] - size,
            },
            sys::ImVec2 {
                x: pos[0],
                y: pos[1] + size,
            },
            col,
            2.0,
        );
    }
}

fn draw_text_shadowed(pos: [f32; 2], font_size: f32, color: ImColor32, text: &str) {
    let shadow = ImColor32::from_rgba(0, 0, 0, 205);
    draw_text([pos[0] + 2.0, pos[1] + 2.0], font_size, shadow, text);
    draw_text(pos, font_size, color, text);
}

fn draw_text(pos: [f32; 2], font_size: f32, color: ImColor32, text: &str) {
    unsafe {
        let draw = sys::igGetForegroundDrawList();
        let font = sys::igGetFont();
        if draw.is_null() || font.is_null() {
            return;
        }

        let text_begin = text.as_ptr() as *const i8;
        let text_end = text_begin.add(text.len());
        let col: u32 = color.into();
        sys::ImDrawList_AddText_FontPtr(
            draw,
            font,
            font_size,
            sys::ImVec2 {
                x: pos[0],
                y: pos[1],
            },
            col,
            text_begin,
            text_end,
            0.0,
            std::ptr::null(),
        );
    }
}

fn normalize_delta(delta: PositionDelta) -> Option<[f32; 3]> {
    let len = (delta.0 * delta.0 + delta.1 * delta.1 + delta.2 * delta.2).sqrt();
    (len.is_finite() && len > 0.001).then_some([delta.0 / len, delta.1 / len, delta.2 / len])
}

fn havok_to_array(pos: HavokPosition) -> [f32; 3] {
    [pos.0, pos.1, pos.2]
}

fn distance_havok(a: HavokPosition, b: HavokPosition) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn length3(value: [f32; 3]) -> f32 {
    (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt()
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
