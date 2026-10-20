use super::*;

#[test]
fn dead_or_disabled_souls_are_not_eligible_for_live_maintenance() {
    assert!(!revenant_soul_finished(1, false, false));
    for (hp, dead, disabled) in [
        (0, false, false),
        (-1, false, false),
        (10, true, false),
        (10, false, true),
    ] {
        assert!(revenant_soul_finished(hp, dead, disabled));
    }
}

#[test]
fn invalidated_passive_bindings_leave_family_health_and_recovery_intact() {
    let now = Instant::now();
    let mut state = ChargeState::new(now);
    state.revenant_summon_hp = [0.0, 0.4, 0.8];
    state.revenant_summon_death_locked = [true, false, false];
    state.revenant_debug_souls.push(RevenantDebugSoul {
        expires_at: now + Duration::from_secs(60),
        requested_at: now,
        handle: Some(FieldInsHandle::none()),
        creator_request_pending: false,
        npc_id: 4205,
        npc_param_id: 42050000,
        hide_at: None,
        character: 1234,
        creator_previous: 0,
    });
    state.revenant_pending_debug_soul = Some(RevenantPendingDebugSoul {
        ready_at: now,
        candidate: RevenantPassiveCandidate {
            handle: FieldInsHandle::none(),
            npc_id: 4205,
            npc_param_id: 42050000,
            npc_think_param_id: 42050000,
            position: HavokPosition::from_xyz(0.0, 0.0, 0.0),
        },
    });
    state.revenant_seen_deaths.insert(FieldInsHandle::none());
    state.invalidate_revenant_passive_bindings();
    assert!(state.revenant_debug_souls.is_empty());
    assert!(state.revenant_pending_debug_soul.is_none());
    assert!(state.revenant_seen_deaths.is_empty());
    assert_eq!(state.revenant_summon_hp, [0.0, 0.4, 0.8]);
    assert_eq!(state.revenant_summon_death_locked, [true, false, false]);
}

#[test]
fn hud_pinned_bindings_match_summon_native_lifecycle_fields() {
    use eldenring::cs::{ChrSet, ChrSetEntry};
    use std::mem::{offset_of, size_of};
    assert_eq!(offset_of!(ChrIns, vftable), 0);
    assert_eq!(offset_of!(ChrIns, field_ins_handle), 8);
    assert_eq!(offset_of!(ChrIns, chr_set_entry), 0x10);
    assert_eq!(offset_of!(ChrIns, event_entity_id), 0x1e8);
    assert_eq!(size_of::<ChrIns>(), 0x580);
    assert_eq!(offset_of!(ChrSet<ChrIns>, capacity), 0x10);
    assert_eq!(offset_of!(ChrSet<ChrIns>, entries), 0x18);
    assert_eq!(offset_of!(ChrSet<ChrIns>, entity_id_mapping), 0x28);
    assert_eq!(offset_of!(ChrSet<ChrIns>, group_id_mapping), 0x40);
    assert_eq!(size_of::<ChrSetEntry<ChrIns>>(), 0x10);
    assert_eq!(offset_of!(ChrSetEntry<ChrIns>, chr_ins), 0);
    assert_eq!(offset_of!(ChrSetEntry<ChrIns>, chr_load_status), 8);
    assert_eq!(offset_of!(ChrSetEntry<ChrIns>, chr_update_type), 9);
    assert_eq!(offset_of!(ChrSetEntry<ChrIns>, entry_flags), 0xa);
    assert_eq!(
        size_of::<ChrSet<ChrIns>>() - offset_of!(ChrSet<ChrIns>, group_id_mapping),
        0x18
    );
    assert_eq!(offset_of!(PlayerIns, locked_on_enemy), 0x6b0);
}
