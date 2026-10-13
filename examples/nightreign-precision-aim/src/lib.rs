use std::{
    ffi::c_void,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        Once,
        atomic::{AtomicIsize, Ordering},
    },
    thread,
    time::Duration,
};

#[path = "../../../sdk_host.rs"]
mod sdk_host;
#[path = "../../../precision_runtime.rs"]
mod sdk_precision;
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{
    ImguiRenderLoop, RenderContext,
    imgui::{Context, ImColor32, TextureId, Ui},
};
use image::io::Reader as ImageReader;
use shikifsrs_elden_ring::cameras::PerspectiveCameraObservation;
use shikifsrs_overlay::hudhook;
use windows::{
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
    },
    core::w,
};

static INSTALL_PRECISION_AIM_TASKS: Once = Once::new();
static DLL_MODULE: AtomicIsize = AtomicIsize::new(0);
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

    thread::spawn(move || {
        for _ in 0..FULL_HUD_DETECTION_POLLS {
            if full_hud_module_loaded() {
                return;
            }
            thread::sleep(FULL_HUD_DETECTION_INTERVAL);
        }

        if let Err(error) = sdk_host::start_overlay(|| Box::new(PrecisionAimHud::default())) {
            tracing::error!(%error, "precision renderer startup failed");
        }
    });

    1
}

fn full_hud_module_loaded() -> bool {
    let _ = FULL_HUD_MODULE_NAME;
    unsafe { GetModuleHandleW(w!("nightreign_style_hud.dll")).is_ok() }
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
        if !sdk_precision::ACTIVE.load(Ordering::Relaxed) {
            return;
        }
        draw_precision_reticle(ui, self.reticles, self.scale);
    }
}

impl PrecisionAimHud {
    fn update_scale(&mut self) {
        self.scale = sdk_host::with_objects(|objects, _| {
            Ok(objects
                .window()?
                .map(|window| (window.configured_width as f32 / 1920.0).clamp(0.65, 1.5)))
        })
        .ok()
        .flatten()
        .unwrap_or(1.0);
    }
}

fn install_precision_aim_tasks_once() {
    INSTALL_PRECISION_AIM_TASKS.call_once(|| {
        std::thread::spawn(|| {
            let configuration = sdk_precision::configure(
                sdk_precision::PrecisionSettings {
                    fov_scale: VISUAL_FOV_SCALE,
                    pullback: VISUAL_CAMERA_PULLBACK,
                    right_offset: VISUAL_CAMERA_RIGHT_OFFSET,
                },
                |effects| sdk_precision::ActivationPolicy {
                    enabled: effects.contains(&ACTIVATION_SPEFFECT),
                    ..Default::default()
                },
            );
            if let Err(error) = configuration.and_then(|_| sdk_precision::install()) {
                tracing::error!(
                    ?error,
                    "precision task registration failed; partial outcomes retained"
                );
            }
        });
    });
}

fn reticle_screen_pos(viewport: [f32; 2]) -> [f32; 2] {
    let center = [viewport[0] * 0.5, viewport[1] * 0.5];
    let Some(camera) = sdk_precision::perspective().ok().flatten() else {
        return center;
    };
    let Some(delta) = sdk_precision::display_delta(camera) else {
        return center;
    };
    let forward: [f32; 3] =
        std::array::from_fn(|index| f32::from_bits(camera.matrix_bits[2][index]));
    let len = dot3(forward, forward).sqrt();
    let aim_point = std::array::from_fn(|index| {
        f32::from_bits(camera.matrix_bits[3][index])
            + delta[index]
            + forward[index] / len * RETICLE_CONVERGENCE_DISTANCE
    });
    project_position(aim_point, camera).unwrap_or(center)
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
            if let Ok(bytes) = std::fs::read(&path)
                && let Some(texture) = load_texture_bytes(render_context, &bytes)
            {
                return Some(texture);
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

fn project_position(position: [f32; 3], camera: PerspectiveCameraObservation) -> Option<[f32; 2]> {
    let extent = sdk_host::with_objects(|objects, _| {
        objects.window()?.map(|v| v.configured_extent()).transpose()
    })
    .ok()
    .flatten()?;
    let width = extent.width as f32;
    let height = extent.height as f32;
    let matrix = camera.matrix_bits.map(|row| row.map(f32::from_bits));
    let rel = std::array::from_fn(|index| position[index] - matrix[3][index]);
    let fov = f32::from_bits(camera.frustum_bits[0]);
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
    let raw_aspect = f32::from_bits(camera.frustum_bits[1]);
    let aspect = if raw_aspect.is_finite() && raw_aspect > 0.0 {
        raw_aspect
    } else {
        width / height
    };
    let axes = (
        std::array::from_fn(|i| matrix[0][i]),
        std::array::from_fn(|i| matrix[1][i]),
        std::array::from_fn(|i| matrix[2][i]),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_and_private_asset_names_are_preserved() {
        assert_eq!(ACTIVATION_SPEFFECT, 883100);
        for (index, files) in RETICLE_FILES.iter().enumerate() {
            assert_eq!(files[0], format!("Precision_Reticle{}.png", index + 1));
            assert_eq!(files[1], format!("Ironeye_Reticle{}.png", index + 1));
        }
    }

    #[test]
    fn embedded_reticle_layers_decode_and_match_dimensions() {
        let images: Vec<_> = EMBEDDED_RETICLES
            .iter()
            .map(|bytes| image::load_from_memory(bytes).unwrap().into_rgba8())
            .collect();
        for image in &images {
            assert_eq!(image.dimensions(), images[0].dimensions());
            assert!(image.width() > 0 && image.height() > 0);
            assert!(image.pixels().any(|pixel| pixel[3] > 0));
        }
    }

    #[test]
    fn convergence_projects_to_center_and_rejects_degenerate_depth() {
        let axes = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(
            project_with_camera_axes(
                [0.0, 0.0, RETICLE_CONVERGENCE_DISTANCE],
                axes,
                1920.0,
                1080.0,
                16.0 / 9.0,
                1.0,
            ),
            Some([960.0, 540.0]),
        );
        assert_eq!(
            project_with_camera_axes([0.0; 3], axes, 1920.0, 1080.0, 16.0 / 9.0, 1.0),
            None,
        );
    }
}
