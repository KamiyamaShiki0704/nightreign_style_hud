use serde::Deserialize;
use std::{
    ffi::{OsString, c_void},
    fs,
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicIsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HINSTANCE, HMODULE},
    System::LibraryLoader::GetModuleFileNameW,
};
mod creation;
mod native;
#[path = "../../../character_runtime.rs"]
mod presentation;
// Exercise the real main-HUD parameter adapter while that member's remaining native APIs are ported.
// This temporary bridge is removed once the main member compiles its own tests.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_parameters.rs"]
mod hud_parameter_contracts;
#[allow(dead_code)] // Shared renderer/query host, separately compiled task-only DLL.
#[path = "../../../sdk_host.rs"]
mod sdk_host;
const DLL_PROCESS_ATTACH: u32 = 1;
const CONFIG_FILE_NAME: &str = "summon.toml";
static DLL_MODULE: AtomicIsize = AtomicIsize::new(0);
static SETTINGS: OnceLock<AttackSummonSettings> = OnceLock::new();
/// # Safety
/// Windows loader provides the module; host retains DLL until worker pins it.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(module: HINSTANCE, reason: u32, _: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        DLL_MODULE.store(module.0 as isize, Ordering::Relaxed);
        thread::spawn(|| {
            let _ = settings();
            if let Err(error) =
                sdk_host::initialize().and_then(|()| native::install().map_err(|e| e.to_string()))
            {
                eprintln!("attack summon initialization rejected: {error}");
            }
        });
    }
    1
}
fn normalize_horizontal(v: (f32, f32, f32), fallback: (f32, f32, f32)) -> (f32, f32, f32) {
    let len = (v.0 * v.0 + v.2 * v.2).sqrt();
    if len <= 0.001 || !len.is_finite() {
        fallback
    } else {
        (v.0 / len, 0.0, v.2 / len)
    }
}
fn summon_position(
    player: [f32; 4],
    forward: (f32, f32, f32),
    right: (f32, f32, f32),
    config: AttackSummonConfig,
    settings: &AttackSummonSettings,
) -> [f32; 4] {
    [
        player[0]
            + forward.0 * config.spawn_forward_distance(settings)
            + right.0 * config.side_offset,
        player[1] + config.spawn_height_offset(settings),
        player[2]
            + forward.2 * config.spawn_forward_distance(settings)
            + right.2 * config.side_offset,
        0.0,
    ]
}
#[derive(Default)]
struct Cooldowns {
    last_any_trigger: Option<Instant>,
    last_trigger_by_slot: Vec<Option<Instant>>,
}
impl Cooldowns {
    fn accept(&mut self, index: usize, now: Instant, settings: &AttackSummonSettings) -> bool {
        self.last_trigger_by_slot
            .resize(settings.summons.len(), None);
        if self.last_any_trigger.is_some_and(|last| {
            now.saturating_duration_since(last) < settings.any_trigger_cooldown()
        }) || self.last_trigger_by_slot[index].is_some_and(|last| {
            now.saturating_duration_since(last) < settings.same_trigger_cooldown()
        }) {
            return false;
        }
        self.last_any_trigger = Some(now);
        self.last_trigger_by_slot[index] = Some(now);
        true
    }
}
#[derive(Clone, Deserialize)]
struct AttackSummonSettings {
    #[serde(default = "default_marker_speffect")]
    marker_speffect: i32,
    #[serde(default = "default_vanish_request_speffect")]
    vanish_request_speffect: i32,
    #[serde(default = "default_generated_team_type")]
    generated_team_type: u8,
    #[serde(default = "default_max_active_summons")]
    max_active_summons: usize,
    #[serde(default = "default_unbound_timeout_ms")]
    unbound_timeout_ms: u64,
    #[serde(default = "default_fallback_lifetime_ms")]
    fallback_lifetime_ms: u64,
    #[serde(default = "default_spawn_forward_distance")]
    spawn_forward_distance: f32,
    #[serde(default = "default_spawn_height_offset")]
    spawn_height_offset: f32,
    #[serde(default = "default_same_trigger_cooldown_ms")]
    same_trigger_cooldown_ms: u64,
    #[serde(default = "default_any_trigger_cooldown_ms")]
    any_trigger_cooldown_ms: u64,
    #[serde(default = "default_reuse_spawned_units")]
    reuse_spawned_units: bool,
    #[serde(default = "default_disable_lock_on")]
    disable_lock_on: bool,
    #[serde(default = "default_summons")]
    summons: Vec<AttackSummonConfig>,
}

impl Default for AttackSummonSettings {
    fn default() -> Self {
        Self {
            marker_speffect: default_marker_speffect(),
            vanish_request_speffect: default_vanish_request_speffect(),
            generated_team_type: default_generated_team_type(),
            max_active_summons: default_max_active_summons(),
            unbound_timeout_ms: default_unbound_timeout_ms(),
            fallback_lifetime_ms: default_fallback_lifetime_ms(),
            spawn_forward_distance: default_spawn_forward_distance(),
            spawn_height_offset: default_spawn_height_offset(),
            same_trigger_cooldown_ms: default_same_trigger_cooldown_ms(),
            any_trigger_cooldown_ms: default_any_trigger_cooldown_ms(),
            reuse_spawned_units: default_reuse_spawned_units(),
            disable_lock_on: default_disable_lock_on(),
            summons: default_summons(),
        }
    }
}

impl AttackSummonSettings {
    fn unbound_timeout(&self) -> Duration {
        Duration::from_millis(self.unbound_timeout_ms.max(1))
    }

    fn same_trigger_cooldown(&self) -> Duration {
        Duration::from_millis(self.same_trigger_cooldown_ms)
    }

    fn any_trigger_cooldown(&self) -> Duration {
        Duration::from_millis(self.any_trigger_cooldown_ms)
    }
}

fn settings() -> &'static AttackSummonSettings {
    SETTINGS.get_or_init(load_settings)
}

fn load_settings() -> AttackSummonSettings {
    let Some(path) = config_path() else {
        return AttackSummonSettings::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return AttackSummonSettings::default();
    };
    let Ok(mut settings) = toml::from_str::<AttackSummonSettings>(&text) else {
        return AttackSummonSettings::default();
    };
    settings.summons.retain(|config| {
        config.trigger_speffect > 0 && config.chr_id > 0 && config.npc_param_id > 0
    });
    if settings.summons.is_empty() {
        settings.summons = default_summons();
    }
    settings
}

fn config_path() -> Option<PathBuf> {
    module_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

fn module_dir() -> Option<PathBuf> {
    let module = DLL_MODULE.load(Ordering::Relaxed);
    if module == 0 {
        return None;
    }

    let mut buffer = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(Some(HMODULE(module as *mut _)), &mut buffer) };
    if len == 0 {
        return None;
    }

    let path = PathBuf::from(OsString::from_wide(&buffer[..len as usize]));
    path.parent().map(Path::to_path_buf)
}

#[derive(Clone, Copy, Deserialize)]
struct AttackSummonConfig {
    trigger_speffect: i32,
    #[serde(default = "default_config_chr_id")]
    chr_id: i32,
    #[serde(default)]
    chara_init_param_id: i32,
    npc_param_id: i32,
    #[serde(default)]
    npc_think_param_id: i32,
    #[serde(default)]
    event_entity_id: i32,
    #[serde(default)]
    talk_id: i32,
    animation_id: i32,
    #[serde(default)]
    side_offset: f32,
    #[serde(default)]
    is_player: bool,
    #[serde(default)]
    marker_speffect: Option<i32>,
    #[serde(default)]
    vanish_request_speffect: Option<i32>,
    #[serde(default)]
    generated_team_type: Option<u8>,
    #[serde(default)]
    fallback_lifetime_ms: Option<u64>,
    #[serde(default)]
    spawn_forward_distance: Option<f32>,
    #[serde(default)]
    spawn_height_offset: Option<f32>,
    #[serde(default)]
    disable_lock_on: Option<bool>,
}

impl AttackSummonConfig {
    fn marker_speffect(&self, settings: &AttackSummonSettings) -> i32 {
        self.marker_speffect.unwrap_or(settings.marker_speffect)
    }

    fn vanish_request_speffect(&self, settings: &AttackSummonSettings) -> i32 {
        self.vanish_request_speffect
            .unwrap_or(settings.vanish_request_speffect)
    }

    fn generated_team_type(&self, settings: &AttackSummonSettings) -> u8 {
        self.generated_team_type
            .unwrap_or(settings.generated_team_type)
    }

    fn fallback_lifetime(&self, settings: &AttackSummonSettings) -> Duration {
        Duration::from_millis(
            self.fallback_lifetime_ms
                .unwrap_or(settings.fallback_lifetime_ms)
                .max(1),
        )
    }

    fn spawn_forward_distance(&self, settings: &AttackSummonSettings) -> f32 {
        self.spawn_forward_distance
            .unwrap_or(settings.spawn_forward_distance)
    }

    fn spawn_height_offset(&self, settings: &AttackSummonSettings) -> f32 {
        self.spawn_height_offset
            .unwrap_or(settings.spawn_height_offset)
    }

    fn disable_lock_on(&self, settings: &AttackSummonSettings) -> bool {
        self.disable_lock_on.unwrap_or(settings.disable_lock_on)
    }
}

fn default_marker_speffect() -> i32 {
    14200128
}

fn default_vanish_request_speffect() -> i32 {
    14200129
}

fn default_generated_team_type() -> u8 {
    47
}

fn default_max_active_summons() -> usize {
    8
}

fn default_unbound_timeout_ms() -> u64 {
    2000
}

fn default_fallback_lifetime_ms() -> u64 {
    10_000
}

fn default_spawn_forward_distance() -> f32 {
    2.8
}

fn default_spawn_height_offset() -> f32 {
    0.05
}

fn default_same_trigger_cooldown_ms() -> u64 {
    350
}

fn default_any_trigger_cooldown_ms() -> u64 {
    120
}

fn default_reuse_spawned_units() -> bool {
    true
}

fn default_disable_lock_on() -> bool {
    false
}

fn default_config_chr_id() -> i32 {
    4205
}

fn default_summons() -> Vec<AttackSummonConfig> {
    vec![
        AttackSummonConfig {
            trigger_speffect: 14200120,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050100,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20010,
            side_offset: -1.25,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
        AttackSummonConfig {
            trigger_speffect: 14200121,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050101,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20011,
            side_offset: -0.75,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
        AttackSummonConfig {
            trigger_speffect: 14200122,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050102,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20012,
            side_offset: -0.25,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
        AttackSummonConfig {
            trigger_speffect: 14200123,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050103,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20013,
            side_offset: 0.25,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
        AttackSummonConfig {
            trigger_speffect: 14200124,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050104,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20017,
            side_offset: 0.75,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
        AttackSummonConfig {
            trigger_speffect: 14200125,
            chr_id: 4205,
            chara_init_param_id: 0,
            npc_param_id: 42050105,
            npc_think_param_id: 42050000,
            event_entity_id: 0,
            talk_id: 0,
            animation_id: 20018,
            side_offset: 1.25,
            is_player: false,
            marker_speffect: None,
            vanish_request_speffect: None,
            generated_team_type: None,
            fallback_lifetime_ms: None,
            spawn_forward_distance: None,
            spawn_height_offset: None,
            disable_lock_on: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_defaults_and_per_slot_overrides_keep_all_summon_inputs() {
        let s = AttackSummonSettings::default();
        assert_eq!(s.summons.len(), 6);
        assert_eq!(
            s.summons
                .iter()
                .map(|v| v.trigger_speffect)
                .collect::<Vec<_>>(),
            (14200120..=14200125).collect::<Vec<_>>()
        );
        assert_eq!(
            s.summons.iter().map(|v| v.animation_id).collect::<Vec<_>>(),
            vec![20010, 20011, 20012, 20013, 20017, 20018]
        );
        let v:AttackSummonConfig=toml::from_str("trigger_speffect=10\nnpc_param_id=20\nanimation_id=-1\nmarker_speffect=30\ngenerated_team_type=5\nfallback_lifetime_ms=0\nspawn_forward_distance=9.0\nspawn_height_offset=4.0\ndisable_lock_on=true\nis_player=true").unwrap();
        assert_eq!(v.marker_speffect(&s), 30);
        assert_eq!(v.generated_team_type(&s), 5);
        assert_eq!(v.fallback_lifetime(&s), Duration::from_millis(1));
        assert!(v.disable_lock_on(&s) && v.is_player);
        assert_eq!(
            summon_position([1., 2., 3., 0.], (0., 0., 1.), (1., 0., 0.), v, &s),
            [1., 6., 12., 0.]
        );
    }
    #[test]
    fn cooldown_boundaries_and_horizontal_fallback_match_original_ordering() {
        let s = AttackSummonSettings::default();
        let t = Instant::now();
        let mut c = Cooldowns::default();
        assert!(c.accept(0, t, &s));
        assert!(!c.accept(
            1,
            t + s.any_trigger_cooldown() - Duration::from_millis(1),
            &s
        ));
        assert!(c.accept(1, t + s.any_trigger_cooldown(), &s));
        assert!(!c.accept(
            0,
            t + s.same_trigger_cooldown() - Duration::from_millis(1),
            &s
        ));
        assert!(c.accept(0, t + s.same_trigger_cooldown(), &s));
        assert_eq!(
            normalize_horizontal((0., 99., 0.), (0., 0., 1.)),
            (0., 0., 1.)
        );
        assert_eq!(
            normalize_horizontal((f32::NAN, 0., 1.), (1., 0., 0.)),
            (1., 0., 0.)
        );
        assert_eq!(
            normalize_horizontal((3., 9., 4.), (0., 0., 1.)),
            (0.6, 0., 0.8)
        );
    }
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_hud.rs"]
mod hud_native_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_handles.rs"]
mod hud_handle_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_tasks.rs"]
mod hud_task_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_hooks.rs"]
mod hud_hook_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../precision_runtime.rs"]
mod hud_precision_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_camera.rs"]
mod hud_camera_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_vitals.rs"]
mod hud_vital_contracts;

#[cfg(test)]
use hud_vital_contracts as native_vitals;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_character.rs"]
mod hud_character_contracts;

#[cfg(test)]
use presentation as character_runtime;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_world.rs"]
mod hud_world_contracts;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_buddy.rs"]
mod hud_buddy_contracts;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_bullets.rs"]
mod hud_bullet_contracts;

#[cfg(test)]
use crate::creation as hud_creation_queue;
// HUD-only entry points are compiled here until its main member finishes porting.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/native_creation.rs"]
mod hud_creation_contracts;

#[cfg(test)]
use hud_handle_contracts as native_handles;
#[cfg(test)]
use hud_world_contracts as native_world;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../nightreign-style-hud/src/actors.rs"]
mod hud_actor_contracts;
