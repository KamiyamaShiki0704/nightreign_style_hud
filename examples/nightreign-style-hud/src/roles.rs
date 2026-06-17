#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Wylder = 0,
    Guardian = 1,
    Ironeye = 2,
    Duchess = 3,
    Raider = 4,
    Revenant = 5,
    Recluse = 6,
    Executor = 7,
    Scholar = 8,
    Undertaker = 9,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum RecluseElement {
    Magic,
    Fire,
    Lightning,
    Holy,
}

impl RecluseElement {
    fn icon_index(self) -> usize {
        match self {
            RecluseElement::Magic => 0,
            RecluseElement::Fire => 1,
            RecluseElement::Lightning => 2,
            RecluseElement::Holy => 3,
        }
    }

    fn mask(self) -> u8 {
        match self {
            RecluseElement::Magic => 0b0001,
            RecluseElement::Fire => 0b0010,
            RecluseElement::Lightning => 0b0100,
            RecluseElement::Holy => 0b1000,
        }
    }

    fn absorb_effect_offset(self) -> i32 {
        match self {
            RecluseElement::Magic => SP_RECLUSE_ABSORB_MAGIC,
            RecluseElement::Fire => SP_RECLUSE_ABSORB_FIRE,
            RecluseElement::Lightning => SP_RECLUSE_ABSORB_LIGHTNING,
            RecluseElement::Holy => SP_RECLUSE_ABSORB_HOLY,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ScholarTargetSnapshot {
    active: bool,
    observed: bool,
    pos: [f32; 2],
    progress: f32,
}

#[derive(Clone, Copy, Default)]
struct ScholarDamageNumberSnapshot {
    active: bool,
    pos: [f32; 2],
    amount: i32,
    age: f32,
    seed: f32,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Default)]
struct MultiLockTargetSnapshot {
    active: bool,
    pos: [f32; 2],
    primary: bool,
}

#[derive(Clone, Copy)]
struct ScholarDamageNumber {
    pos: [f32; 2],
    amount: i32,
    created_at: Instant,
    seed: f32,
}

#[derive(Clone, Copy, Default)]
struct IroneyeWeaknessSnapshot {
    active: bool,
    pos: [f32; 2],
    progress: f32,
    remaining: f32,
}

#[derive(Clone, Copy, Default)]
struct IroneyeWeaknessBurstSnapshot {
    active: bool,
    pos: [f32; 2],
    age: f32,
}

#[derive(Clone, Copy)]
struct IroneyeWeaknessMark {
    damage_module: usize,
    accumulated_damage: i32,
    threshold_damage: i32,
    started_at: Instant,
    expires_at: Instant,
}

#[derive(Clone, Copy)]
struct IroneyeWeaknessBurst {
    pos: [f32; 2],
    created_at: Instant,
}

#[derive(Clone, Copy, Default)]
struct ScholarTargetState {
    progress: f32,
    pos: [f32; 2],
    visible: bool,
    handle: Option<FieldInsHandle>,
}

impl ScholarTargetState {
    fn stage(self) -> Option<usize> {
        scholar_progress_stage(self.progress)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScholarLinkKind {
    SelfLink,
    Ally,
    Enemy,
}

#[derive(Clone, Copy)]
struct ScholarLink {
    handle: FieldInsHandle,
    kind: ScholarLinkKind,
    hp: i32,
    last_hit_by: Option<FieldInsHandle>,
}

#[derive(Clone, Copy, Default)]
struct ScholarScanDebug {
    scanned: usize,
    type_skip: usize,
    hidden_skip: usize,
    hp_skip: usize,
    screen_skip: usize,
    range_skip: usize,
    los_skip: usize,
    accepted: usize,
    candidate: ScholarCandidateDebug,
}

#[derive(Clone, Copy, Default)]
struct ScholarCandidateDebug {
    active: bool,
    selector_index: u32,
    selector_container: u32,
    block_area: u8,
    block_block: u8,
    block_region: u8,
    block_index: u8,
    npc_id: i32,
    chr_type: i32,
    hp: i32,
    max_hp: i32,
    distance: f32,
    load_status: u8,
    chr_update_type: u8,
    draw_group: bool,
    backread_disabled: bool,
    host_inactive: bool,
    extinction_death: bool,
    near_pc: bool,
    render_group: bool,
    onscreen_flag: bool,
    enable_render: bool,
    death_flag: bool,
    is_active: bool,
    update_tasks_registered: bool,
    force_unloaded: bool,
    character_disabled: bool,
    tint_alpha: f32,
    tint_alpha_modifier: f32,
    base_transparency: f32,
    base_transparency_modifier: f32,
    event_entity_id: u32,
    chr_set_entry_flags: u8,
    activation_enabled: bool,
    activate_threshold_exceeded: bool,
    sounds_active: bool,
    opacity_keyframes: f32,
    opacity_keyframes_previous: f32,
    camouflage_transparency: f32,
    distance_to_player_sqr: f32,
    horizontal_distance_to_player_sqr: f32,
    max_render_range: f32,
    chr_activate_threshold: f32,
    current_anim_id: i32,
    request_anim_id: i32,
    idle_anim_id: i32,
    current_tae_id: i32,
    chr_collision: usize,
    ctrl_disable_move: bool,
    ctrl_disable_player_collision: bool,
    ctrl_disable_hit: bool,
    ctrl_disable_map_collision: bool,
    ctrl_disable_capsule_collision: bool,
    ctrl_disable_object_collision: bool,
    ctrl_proxy_pos_sync: bool,
    ctrl_proxy_rot_sync: bool,
    physics_chr_proxy: usize,
    physics_chr_proxy2: usize,
    physics_collision_shape: usize,
    physics_pos_update_requested: bool,
    physics_standing_on_ground: bool,
    physics_touching_ground: bool,
    physics_slide_enabled: bool,
    physics_gravity_disabled: bool,
    chr_ptr: usize,
    data_ptr: usize,
    chr_set_entry_ptr: usize,
    msb_draw_flags: u32,
    msb_part_ptr: usize,
    chr_hash: u32,
    data_hash: u32,
    msb_hash: u32,
    entry_hash: u32,
    entry_raw0: u64,
    entry_raw1: u64,
    los_target_distance: f32,
    los_hit_mask: u32,
    los_near_mask: u32,
    los_block_mask: u32,
    los_closest_hit_distance: f32,
    los_block_filter: i32,
}

#[derive(Clone, Copy)]
enum ScholarRejectReason {
    Type,
    Hidden,
    Hp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SkillKind {
    Timed,
    NoCooldown,
    UndertakerBuff,
}

#[derive(Clone, Copy)]
struct RoleConfig {
    role: Role,
    sp_effect: i32,
    skill_kind: SkillKind,
    skill_cooldown: f32,
    skill_charges: usize,
    ultimate_cooldown: f32,
    discounted_ultimate: bool,
    ultimate_full_cost: f32,
    skill_bytes: &'static [u8],
    ultimate_bytes: &'static [u8],
}

const ROLE_COUNT: usize = 10;
const ULTIMATE_HIT_GAIN: f32 = 0.008;
const ULTIMATE_BIG_GAIN: f32 = 0.05;
const SKILL_CONSUME_INTERVAL: f32 = 0.5;
const ULTIMATE_HIT_GAIN_INTERVAL: f32 = 0.1;
const ULTIMATE_KILL_GAIN_INTERVAL: f32 = 0.1;
const ULTIMATE_CRITICAL_GAIN_INTERVAL: f32 = 0.5;
const RECLUSE_ABSORB_INTERVAL: f32 = 0.3;
const SCHOLAR_SCAN_GAIN_MIN: f32 = 0.055;
const SCHOLAR_SCAN_GAIN_MAX: f32 = 1.0 / 2.4;
const SCHOLAR_SCAN_DECAY: f32 = 0.08;
const SCHOLAR_SCAN_MAX_TARGETS: usize = 16;
const SCHOLAR_SCAN_MAX_DISTANCE_ENTRIES: usize = 192;
const SCHOLAR_SCAN_DEBUG_VISIBLE: bool = false;
const SCHOLAR_SCAN_CLOSE_DISTANCE: f32 = 4.0;
const SCHOLAR_DAMAGE_NUMBER_MAX: usize = 16;
const IRONEYE_WEAKNESS_MAX_TARGETS: usize = 16;
const IRONEYE_WEAKNESS_MAX_BURSTS: usize = 8;
const IRONEYE_WEAKNESS_DURATION: f32 = 17.0;
const IRONEYE_WEAKNESS_THRESHOLD_MAX_HP_RATE: f32 = 0.20;
const IRONEYE_WEAKNESS_FLASH_SECONDS: f32 = 3.0;
const IRONEYE_WEAKNESS_MARK_SCALE: f32 = 5.0;
const IRONEYE_WEAKNESS_BURST_SPREAD_SECONDS: f32 = 0.35;
const IRONEYE_WEAKNESS_BURST_HOLD_SECONDS: f32 = 0.25;
const IRONEYE_WEAKNESS_BURST_FADE_SECONDS: f32 = 0.25;
const IRONEYE_WEAKNESS_BURST_SECONDS: f32 =
    IRONEYE_WEAKNESS_BURST_SPREAD_SECONDS
        + IRONEYE_WEAKNESS_BURST_HOLD_SECONDS
        + IRONEYE_WEAKNESS_BURST_FADE_SECONDS;
const IRONEYE_RETICLE_HALF_SIZE: f32 = 46.0;
const SCHOLAR_SCAN_MAX_DISTANCE: f32 = 58.0;
const SCHOLAR_SCAN_DISTANCE_EXPONENT: f32 = 1.65;
const SCHOLAR_LOCK_TARGET_FALLBACK_RADIUS: f32 = 38.0;
const SCHOLAR_LOCK_TARGET_RADIUS_SCALE: f32 = 1.15;
const SCHOLAR_LOCK_TARGET_MAX_VERTICAL: f32 = 12.0;
const SCHOLAR_LOCK_TARGET_MAX_SIDE_FACTOR: f32 = 0.82;
const SCHOLAR_LOCK_TARGET_SIDE_DEPTH_FACTOR: f32 = 0.85;
const SCHOLAR_LOCK_TARGET_MIN_CAMERA_DEPTH: f32 = 1.0;
const SCHOLAR_LOCK_TARGET_FORWARD_PAD: f32 = 4.0;
const SCHOLAR_LINE_OF_SIGHT_FILTERS: [u32; 9] = [0, 1, 2, 4, 8, 16, 32, 64, 128];
const SCHOLAR_LINE_OF_SIGHT_NEAR_HIT_IGNORE: f32 = 0.65;
const SCHOLAR_LINE_OF_SIGHT_MIN_TOLERANCE: f32 = 3.5;
const SCHOLAR_LINE_OF_SIGHT_RADIUS_TOLERANCE: f32 = 3.0;
const SCHOLAR_LINE_OF_SIGHT_HEIGHT_TOLERANCE: f32 = 0.25;
const SCHOLAR_LENS_CENTER_X_FACTOR: f32 = 0.48;
const SCHOLAR_LENS_RADIUS_FACTOR: f32 = 0.715;
const SCHOLAR_SCAN_RADIUS_FACTOR: f32 = 0.63;

const SHARED_EFFECT_BASE: i32 = 880000;
const SP_SKILL_READY_1: i32 = 1;
const SP_SKILL_READY_2: i32 = 2;
const SP_ULTIMATE_READY_LEGACY_70: i32 = 3;
const SP_ULTIMATE_READY_LEGACY_100: i32 = 4;
const SP_ULTIMATE_READY: i32 = 5;
const SP_SKILL_USED: i32 = 11;
const SP_SKILL_USED_SECOND: i32 = 12;
const SP_SKILL_USED_ENHANCED: i32 = 13;
const SP_SKILL_CANCEL: i32 = 14;
const SP_UNDERTAKER_SKILL_AUTO_END: i32 = 15;
const SP_UNDERTAKER_ULTIMATE_FREE_TRIGGER: i32 = 16;
const SP_IRONEYE_WEAKNESS_TRIGGER: i32 = 17;
const SP_RECLUSE_RELEASED: i32 = 18;
const SP_RECLUSE_RESTORE_FP: i32 = 19;
const SP_IRONEYE_WEAKNESS_ACTIVE: i32 = 18;
const SP_IRONEYE_WEAKNESS_BREAK: i32 = 19;
const SP_ULTIMATE_CHARGED_USED: i32 = 21;
const SP_ULTIMATE_UNCHARGED_USED: i32 = 22;
const SP_EXECUTOR_ULTIMATE_CANCEL: i32 = 24;
const SP_EXECUTOR_ULTIMATE_AUTO_END: i32 = 25;
const SP_ULTIMATE_KILL_GAIN: i32 = 26;
const SP_ULTIMATE_CRITICAL_GAIN: i32 = 27;
const SP_RECLUSE_MIXED_MAGIC_BASE: i32 = 31;
const SP_RECLUSE_ABSORB_MAGIC: i32 = 51;
const SP_RECLUSE_ABSORB_FIRE: i32 = 52;
const SP_RECLUSE_ABSORB_LIGHTNING: i32 = 53;
const SP_RECLUSE_ABSORB_HOLY: i32 = 54;
const SP_SCHOLAR_OBSERVE: i32 = 30;
const SP_SCHOLAR_APPLY_SELF: i32 = 31;
const SP_SCHOLAR_APPLY_ENEMY: i32 = 32;
const SP_SCHOLAR_ENEMY_STAGE_BASE: i32 = 41;
const SP_SCHOLAR_SELF_STAGE_BASE: i32 = 46;
const SP_SCHOLAR_SYMPATHY_SELF: i32 = 60;
const SP_SCHOLAR_SYMPATHY_ALLY: i32 = 61;
const SP_SCHOLAR_SYMPATHY_ENEMY: i32 = 62;
const SCHOLAR_SYMPATHY_DAMAGE_SPREAD_RATE: f32 = 0.20;
const SCHOLAR_SYMPATHY_ATTACK_HEAL_RATE: f32 = 0.30;
const SCHOLAR_SYMPATHY_COUNTER_RATE: f32 = 0.25;
const SCHOLAR_SYMPATHY_HEAL_SPREAD_RATE: f32 = 0.20;
const SCHOLAR_SYMPATHY_HEAL_DAMAGE_RATE: f32 = 0.20;
const SCHOLAR_DAMAGE_NUMBER_SECONDS: f32 = 1.25;
const SCHOLAR_SPEFFECT_DAMAGE_IGNORE_SECONDS: f32 = 0.75;
const SP_SCHOLAR_SYMPATHY_DAMAGE: i32 = 880865;
const DUCHESS_REPLAY_RADIUS: f32 = 8.0;
const DUCHESS_REPLAY_RATE: f32 = 0.50;
const SP_DUCHESS_REPLAY_DAMAGE: i32 = 880365;
// TODO: Duchess replay visual needs a real ReplayGhost/ghost_chr_set path, not HUD afterimages or SpEffect tinting.
const RECLUSE_TRANSIENT_OUTPUT_SECONDS: f32 = 0.35;
const REVENANT_SUMMON_COUNT: usize = 3;
const REVENANT_SUMMON_EFFECTS: [i32; REVENANT_SUMMON_COUNT] = [270000, 271000, 272000];
const REVENANT_BUDDY_PARAM_IDS: [u32; REVENANT_SUMMON_COUNT] = [27000000, 27100000, 27200000];
const REVENANT_FAMILY_REGEN_PER_SECOND: f32 = 0.02;
const REVENANT_SUMMON_HP_RESTORE_SECONDS: f32 = 0.8;

const UNDERTAKER_BUFF_SECONDS: f32 = 8.0;
const UNDERTAKER_ENHANCED_BUFF_SECONDS: f32 = 15.0;
const UNDERTAKER_FREE_ULTIMATE_SECONDS: f32 = 4.0;
const EXECUTOR_TRANSFORM_SECONDS: f32 = 15.0;

const ROLE_CONFIGS: [RoleConfig; ROLE_COUNT] = [
    RoleConfig {
        role: Role::Wylder,
        sp_effect: 880000,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 8.0,
        skill_charges: 2,
        ultimate_cooldown: 335.0,
        discounted_ultimate: true,
        ultimate_full_cost: 0.7,
        skill_bytes: include_bytes!("../assets/Wylder_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Wylder_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Guardian,
        sp_effect: 880100,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 14.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Guardian_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Guardian_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Ironeye,
        sp_effect: 880200,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 10.0,
        skill_charges: 2,
        ultimate_cooldown: 335.0,
        discounted_ultimate: true,
        ultimate_full_cost: 0.7,
        skill_bytes: include_bytes!("../assets/Ironeye_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Ironeye_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Duchess,
        sp_effect: 880300,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 12.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Duchess_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Duchess_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Raider,
        sp_effect: 880400,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 12.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Raider_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Raider_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Revenant,
        sp_effect: 880500,
        skill_kind: SkillKind::NoCooldown,
        skill_cooldown: 0.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Revenant_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Revenant_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Recluse,
        sp_effect: 880600,
        skill_kind: SkillKind::NoCooldown,
        skill_cooldown: 0.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Recluse_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Recluse_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Executor,
        sp_effect: 880700,
        skill_kind: SkillKind::NoCooldown,
        skill_cooldown: 0.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Executor_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Executor_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Scholar,
        sp_effect: 880800,
        skill_kind: SkillKind::Timed,
        skill_cooldown: 12.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: false,
        ultimate_full_cost: 1.0,
        skill_bytes: include_bytes!("../assets/Scholar_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Scholar_Ultimate.png"),
    },
    RoleConfig {
        role: Role::Undertaker,
        sp_effect: 880900,
        skill_kind: SkillKind::UndertakerBuff,
        skill_cooldown: 14.0,
        skill_charges: 1,
        ultimate_cooldown: 335.0,
        discounted_ultimate: true,
        ultimate_full_cost: 0.7,
        skill_bytes: include_bytes!("../assets/Undertaker_Skill.png"),
        ultimate_bytes: include_bytes!("../assets/Undertaker_Ultimate.png"),
    },
];

impl RoleConfig {
    fn effect(&self, offset: i32) -> i32 {
        self.sp_effect + offset
    }
}

fn shared_effect(offset: i32) -> i32 {
    SHARED_EFFECT_BASE + offset
}

