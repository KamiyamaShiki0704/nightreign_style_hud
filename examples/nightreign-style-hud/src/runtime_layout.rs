const CHR_DEBUG_DISABLED_HIT: u32 = native_character::DEBUG_DISABLED_HIT;
const CHR_DEBUG_DISABLED_SECONDARY_ACTIONS: u32 =
    native_character::DEBUG_DISABLED_SECONDARY_ACTIONS;
const CHR_DEBUG_DISABLED_MOVEMENT: u32 = native_character::DEBUG_DISABLED_MOVEMENT;
const CHR_DEBUG_DISABLED_UPDATES: u32 = native_character::DEBUG_DISABLED_UPDATES;
const CHR_DEBUG_FORCE_UNLOADED: u32 = native_character::DEBUG_FORCE_UNLOADED;
const CHR_DEBUG_FORCE_LOADED: u32 = native_character::DEBUG_FORCE_LOADED;
const CHR_DEBUG_CHARACTER_DISABLED: u32 = native_character::DEBUG_CHARACTER_DISABLED;

fn runtime_chr_force_unloaded(chr: &HudActor) -> bool {
    native_character::debug_requests(chr.identity.address).map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD debug request observation rejected: {error}");
            true
        },
        |value| value.0,
    )
}
fn runtime_chr_character_disabled(chr: &HudActor) -> bool {
    native_character::debug_requests(chr.identity.address).map_or_else(
        |error| {
            eprintln!("ShikiFSRS HUD debug request observation rejected: {error}");
            true
        },
        |value| value.1,
    )
}
#[cfg(test)]
mod runtime_layout_tests {
    use super::*;

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

fn hud_physics_position(chr: &HudActor) -> HudPosition {
    match native_character::position(chr.identity.address) {
        Ok(bits) => {
            let [x, y, z, w] = bits.map(f32::from_bits);
            HudPosition(x, y, z, w)
        }
        Err(error) => {
            eprintln!("ShikiFSRS HUD spatial copy rejected: {error}");
            // Invalid math signal: projection/query/spawn admission must reject it.
            // This never describes a valid copied game position or native allocation.
            HudPosition(f32::NAN, f32::NAN, f32::NAN, f32::NAN)
        }
    }
}
