const ER_270_BULLET_SPAWN_RVA: u32 = 0x3a2cb0;

const CHR_DEBUG_FLAGS_PRE_117_OFFSET: usize = 0x530;
const CHR_DEBUG_FLAGS_117_OFFSET: usize = 0x538;
const ENEMY_NPC_THINK_PRE_117_OFFSET: usize = 0x598;
const ENEMY_NPC_THINK_117_OFFSET: usize = 0x5a0;

const CHR_DISTANCE_TO_PLAYER_SQR_PRE_117_OFFSET: usize = 0x3f4;
const CHR_DISTANCE_TO_PLAYER_SQR_117_OFFSET: usize = 0x3fc;
const CHR_HORIZONTAL_DISTANCE_TO_PLAYER_SQR_PRE_117_OFFSET: usize = 0x3f8;
const CHR_HORIZONTAL_DISTANCE_TO_PLAYER_SQR_117_OFFSET: usize = 0x400;
const CHR_MAX_RENDER_RANGE_PRE_117_OFFSET: usize = 0x400;
const CHR_MAX_RENDER_RANGE_117_OFFSET: usize = 0x408;
const CHR_ACTIVATE_THRESHOLD_PRE_117_OFFSET: usize = 0x404;
const CHR_ACTIVATE_THRESHOLD_117_OFFSET: usize = 0x40c;

const CHR_DEBUG_DISABLED_HIT: u32 = 1 << 3;
const CHR_DEBUG_DISABLED_SECONDARY_ACTIONS: u32 = 1 << 4;
const CHR_DEBUG_DISABLED_MOVEMENT: u32 = 1 << 5;
const CHR_DEBUG_DISABLED_UPDATES: u32 = 1 << 8;
const CHR_DEBUG_FORCE_UNLOADED: u32 = 1 << 10;
const CHR_DEBUG_FORCE_LOADED: u32 = 1 << 11;
const CHR_DEBUG_CHARACTER_DISABLED: u32 = 1 << 12;

fn uses_er_117_chr_layout() -> bool {
    bullet_spawn_rva() == ER_270_BULLET_SPAWN_RVA
}

const fn versioned_chr_offset(pre_117: usize, er_117: usize, uses_er_117: bool) -> usize {
    if uses_er_117 { er_117 } else { pre_117 }
}

fn read_chr_u32(chr: &ChrIns, pre_117: usize, er_117: usize) -> u32 {
    let offset = versioned_chr_offset(pre_117, er_117, uses_er_117_chr_layout());
    unsafe {
        std::ptr::read_unaligned(
            std::ptr::from_ref(chr)
                .cast::<u8>()
                .add(offset)
                .cast::<u32>(),
        )
    }
}

fn read_chr_f32(chr: &ChrIns, pre_117: usize, er_117: usize) -> f32 {
    f32::from_bits(read_chr_u32(chr, pre_117, er_117))
}

fn runtime_chr_debug_flags(chr: &ChrIns) -> u32 {
    read_chr_u32(
        chr,
        CHR_DEBUG_FLAGS_PRE_117_OFFSET,
        CHR_DEBUG_FLAGS_117_OFFSET,
    )
}

fn runtime_chr_force_unloaded(chr: &ChrIns) -> bool {
    runtime_chr_debug_flags(chr) & CHR_DEBUG_FORCE_UNLOADED != 0
}

fn runtime_chr_character_disabled(chr: &ChrIns) -> bool {
    runtime_chr_debug_flags(chr) & CHR_DEBUG_CHARACTER_DISABLED != 0
}

fn update_runtime_chr_debug_flags(chr: &mut ChrIns, set_mask: u32, clear_mask: u32) {
    let offset = versioned_chr_offset(
        CHR_DEBUG_FLAGS_PRE_117_OFFSET,
        CHR_DEBUG_FLAGS_117_OFFSET,
        uses_er_117_chr_layout(),
    );
    unsafe {
        let flags = std::ptr::from_mut(chr)
            .cast::<u8>()
            .add(offset)
            .cast::<u32>();
        let value = std::ptr::read_unaligned(flags);
        std::ptr::write_unaligned(flags, (value | set_mask) & !clear_mask);
    }
}

fn runtime_enemy_npc_think_param(enemy: &EnemyIns) -> i32 {
    let offset = versioned_chr_offset(
        ENEMY_NPC_THINK_PRE_117_OFFSET,
        ENEMY_NPC_THINK_117_OFFSET,
        uses_er_117_chr_layout(),
    );
    unsafe {
        std::ptr::read_unaligned(
            std::ptr::from_ref(enemy)
                .cast::<u8>()
                .add(offset)
                .cast::<i32>(),
        )
    }
}

fn runtime_chr_distance_to_player_sqr(chr: &ChrIns) -> f32 {
    read_chr_f32(
        chr,
        CHR_DISTANCE_TO_PLAYER_SQR_PRE_117_OFFSET,
        CHR_DISTANCE_TO_PLAYER_SQR_117_OFFSET,
    )
}

fn runtime_chr_horizontal_distance_to_player_sqr(chr: &ChrIns) -> f32 {
    read_chr_f32(
        chr,
        CHR_HORIZONTAL_DISTANCE_TO_PLAYER_SQR_PRE_117_OFFSET,
        CHR_HORIZONTAL_DISTANCE_TO_PLAYER_SQR_117_OFFSET,
    )
}

fn runtime_chr_max_render_range(chr: &ChrIns) -> f32 {
    read_chr_f32(
        chr,
        CHR_MAX_RENDER_RANGE_PRE_117_OFFSET,
        CHR_MAX_RENDER_RANGE_117_OFFSET,
    )
}

fn runtime_chr_activate_threshold(chr: &ChrIns) -> f32 {
    read_chr_f32(
        chr,
        CHR_ACTIVATE_THRESHOLD_PRE_117_OFFSET,
        CHR_ACTIVATE_THRESHOLD_117_OFFSET,
    )
}

#[cfg(test)]
mod runtime_layout_tests {
    use super::*;

    #[test]
    fn selects_the_er_117_tail_shift_only_for_the_new_layout() {
        assert_eq!(versioned_chr_offset(0x530, 0x538, false), 0x530);
        assert_eq!(versioned_chr_offset(0x530, 0x538, true), 0x538);
    }

    #[test]
    fn er_117_offsets_match_the_verified_runtime_layout() {
        assert_eq!(CHR_DEBUG_FLAGS_117_OFFSET, 0x538);
        assert_eq!(ENEMY_NPC_THINK_117_OFFSET, 0x5a0);
        assert_eq!(CHR_DISTANCE_TO_PLAYER_SQR_117_OFFSET, 0x3fc);
        assert_eq!(CHR_HORIZONTAL_DISTANCE_TO_PLAYER_SQR_117_OFFSET, 0x400);
        assert_eq!(CHR_MAX_RENDER_RANGE_117_OFFSET, 0x408);
        assert_eq!(CHR_ACTIVATE_THRESHOLD_117_OFFSET, 0x40c);
    }

    #[test]
    fn revenant_debug_flag_masks_do_not_overlap() {
        let flags = CHR_DEBUG_DISABLED_HIT
            | CHR_DEBUG_DISABLED_SECONDARY_ACTIONS
            | CHR_DEBUG_DISABLED_MOVEMENT
            | CHR_DEBUG_DISABLED_UPDATES
            | CHR_DEBUG_FORCE_UNLOADED
            | CHR_DEBUG_FORCE_LOADED
            | CHR_DEBUG_CHARACTER_DISABLED;
        assert_eq!(flags.count_ones(), 7);
    }
}
