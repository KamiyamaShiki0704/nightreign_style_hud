use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void};
use std::io::Cursor;
use std::sync::{
    LazyLock, Mutex, Once,
    atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use eldenring::{
    cs::{
        BuddyParam, BuddyStoneParam, BulletSpawnData, CSBulletManager, CSCamExt, CSCamera,
        CSFeManHudState, CSFeManImp, CSHavokMan, CSNowLoadingHelper, CSTaskGroupIndex, CSTaskImp,
        CSWindowImp, ChrDebugSpawnRequest, ChrIns, ChrInsExt, ChrType, EnemyIns, FieldInsHandle,
        FieldInsType, GameDataMan, GameMan, LockCamParam, PlayerIns, SoloParamRepository,
        SpEffectParam, WorldChrMan,
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

include!("damage.rs");
include!("hud.rs");
include!("roles.rs");
include!("state.rs");
include!("runtime.rs");
include!("icons.rs");
include!("draw.rs");
