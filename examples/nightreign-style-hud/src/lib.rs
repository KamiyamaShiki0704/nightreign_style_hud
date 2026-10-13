use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void};
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::{
    LazyLock, Mutex, Once,
    atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

mod actors;
use actors::{HudActor, HudDelta, HudPlayer, HudPosition, HudWorld, hud_actor_by_handle};
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{
    ImguiRenderLoop, RenderContext,
    imgui::{Context, ImColor32, TextureId, Ui, sys},
};
use image::io::Reader as ImageReader;
#[path = "../../../character_runtime.rs"]
#[allow(
    dead_code,
    reason = "Shared adapter APIs are exercised by the attack-summon workspace member"
)]
mod character_runtime;
#[path = "../../nightreign-attack-summon/src/creation.rs"]
#[allow(
    dead_code,
    reason = "Shared adapter APIs are exercised by the attack-summon workspace member"
)]
mod hud_creation_queue;
mod native_buddy;
mod native_bullets;
mod native_camera;
mod native_character;
mod native_creation;
mod native_handles;
mod native_hooks;
mod native_hud;
mod native_tasks;
mod native_vitals;
mod native_world;
use native_handles::HudHandle;
mod native_parameters;
#[path = "../../../sdk_host.rs"]
mod sdk_host;
#[path = "../../../precision_runtime.rs"]
mod sdk_precision;

include!("runtime_layout.rs");
include!("damage.rs");
include!("hud.rs");
include!("roles.rs");
include!("state.rs");
include!("runtime.rs");
include!("icons.rs");
include!("draw.rs");
