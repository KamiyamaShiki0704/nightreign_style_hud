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

use eldenring::{
    cs::{
        AtkParam_Npc, AtkParam_Pc, BuddyParam, BuddyStoneParam, BulletSpawnData, CSBulletManager,
        CSCamExt, CSCamera, CSFeManHudState, CSFeManImp, CSHavokMan, CSNowLoadingHelper,
        CSTaskGroupIndex, CSTaskImp, CSWindowImp, ChrDebugSpawnRequest, ChrIns, ChrInsExt, ChrType,
        EnemyIns, FieldInsHandle, FieldInsType, GameDataMan, GameMan, LockCamParam, NpcParam,
        NpcThinkParam, PlayerIns, SoloParamRepository, SpEffectParam, WorldChrMan,
        bullet_spawn_rva,
    },
    fd4::FD4TaskData,
    position::{DirectionalVector, HavokPosition, PositionDelta},
    util::system::wait_for_system_init,
};
use fromsoftware_shared::{FromStatic, SharedTaskImpExt, Superclass, program::Program};
use hudhook::windows::Win32::Foundation::HINSTANCE;
use hudhook::{
    ImguiRenderLoop, RenderContext,
    hooks::dx12::ImguiDx12Hooks,
    imgui::{Context, ImColor32, TextureId, Ui, sys},
};
use image::io::Reader as ImageReader;
use pelite::pe64::{Pe, PeObject};

// Native implementation pinned from ER_Summon 01491ed; see retirement design.
#[allow(dead_code)]
mod chr_lifecycle;
mod loading_state;
mod retirement;
mod revenant_cleanup;
#[cfg(test)]
mod revenant_cleanup_tests;

fn log_line(message: impl std::fmt::Display) {
    revenant_cleanup::log(message);
}

include!("runtime_layout.rs");
include!("received_direction.rs");
include!("damage.rs");
include!("hud.rs");
include!("roles.rs");
include!("state.rs");
include!("runtime.rs");
include!("icons.rs");
include!("draw.rs");
