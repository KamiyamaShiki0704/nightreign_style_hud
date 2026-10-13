//! Selected copied character observations. These values never borrow native storage.
use crate::{native_vitals, sdk_host};
use shikifsrs_elden_ring::{
    character_modules::CharacterModule, character_physics::CharacterPhysicsObservation,
};
use shikifsrs_runtime::{Error, Result};

pub fn initializer_override(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<i32> {
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD initializer override epoch changed",
            ));
        }
        objects
            .character_initializer_override(target)?
            .map(|copy| copy.stored_id)
            .ok_or(Error::NativeOperationRejected(
                "HUD initializer override data absent",
            ))
    })
}

/// Consumer interaction index 4 is independently assigned by the installed
/// common_define.hks (LOADK/SETGLOBAL at 0x32c/0x330). This copies the current
/// request word, not its edge mask and not a completed interaction.
pub fn action_requested(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
    index: u8,
) -> Result<bool> {
    if index >= 64 {
        return Err(Error::NativeOperationRejected(
            "HUD action bit out of range",
        ));
    }
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD action request epoch changed",
            ));
        }
        objects
            .character_actions(target)?
            .map(|copy| copy.words.action_requests & (1u64 << index) != 0)
            .ok_or(Error::NativeOperationRejected(
                "HUD action request module absent",
            ))
    })
}

/// # Safety
/// Pin this character's complete physics/controller graph, the exclusive writable update
/// phase and every transitive engine reader until both journaled postflights complete.
pub unsafe fn publish_primary_position(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
    position: [u32; 4],
) -> Result<()> {
    if sdk_host::epoch() != target.epoch {
        return Err(Error::NativeOperationRejected(
            "HUD position publication epoch changed",
        ));
    }
    mutate(target.address, |access, record| {
        // SAFETY: Caller supplies the full live graph/phase; actual receipts remain in the journal.
        unsafe { access.publish_primary_position(record, position) }
    })
}

pub fn entry_state(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<shikifsrs_elden_ring::character_sets::CharacterEntryStateObservation> {
    sdk_host::with_objects(|objects, epoch| {
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected(
                "HUD entry-state epoch changed",
            ));
        }
        objects
            .character_entry_state(target)?
            .ok_or(Error::NativeOperationRejected(
                "HUD reciprocal character entry absent",
            ))
    })
}

pub fn kind_record(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<shikifsrs_elden_ring::character_kind_records::CharacterKindRecordObservation> {
    sdk_host::with_objects(|objects, epoch| {
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected(
                "HUD kind-record epoch changed",
            ));
        }
        objects
            .character_kind_record(target)?
            .ok_or(Error::NativeOperationRejected(
                "HUD kind-record character absent",
            ))
    })
}

/// Admission observes the actual modifier word; absence never authorizes lock-on.
pub fn lock_target_suppression(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<bool> {
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD lock-policy epoch changed",
            ));
        }
        objects
            .controller_policy(target)?
            .and_then(|policy| policy.lock_target_suppression_requested())
            .ok_or(Error::NativeOperationRejected(
                "HUD lock-policy modifier absent",
            ))
    })
}

/// Consumer friendship policy over copied fields; this does not query network ownership.
/// ER EMEDF TargetType supplies 0/1/17. Native 50c6d0 treats raw 1/19 together
/// before 3f7490; accepting 19 here remains an experimental consumer inference.
pub fn friendly_to_team(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
    player_team: u8,
    converted_team: u8,
) -> Result<bool> {
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD friendship epoch changed",
            ));
        }
        let flags = objects
            .character_flags(target)?
            .ok_or(Error::NativeOperationRejected(
                "HUD friendship flags absent",
            ))?;
        let selection = objects
            .character_selection(target)?
            .ok_or(Error::NativeOperationRejected("HUD friendship kind absent"))?;
        if objects.character_flags(target)? != Some(flags)
            || objects.character_selection(target)? != Some(selection)
        {
            return Err(Error::NativeOperationRejected(
                "HUD friendship fields changed",
            ));
        }
        let team =
            flags.value(shikifsrs_elden_ring::character_flags::CharacterFlagByte::StoredTeam);
        Ok((player_team != 0 && team == player_team)
            || team == converted_team
            || matches!(selection.raw_kind, 0 | 1 | 17 | 19))
    })
}

pub fn initializer_word(
    address: u64,
    field: shikifsrs_elden_ring::character_parameters::CharacterInitializerWord,
) -> Result<Option<i32>> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        Ok(objects
            .character_initializer_word(target, field)?
            .map(|word| word.value))
    })
}

pub fn physics(address: u64) -> Result<CharacterPhysicsObservation> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_physics(target)?
            .ok_or(Error::NativeOperationRejected("HUD physics module absent"))
    })
}
pub fn position(address: u64) -> Result<[u32; 4]> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_spatial(target)?
            .map(|spatial| spatial.position_bits)
            .ok_or(Error::NativeOperationRejected("HUD spatial module absent"))
    })
}
#[derive(Debug, Clone, Copy)]
pub struct PhysicsShape {
    pub position_bits: [u32; 4],
    pub height: f32,
    pub radius: f32,
}
pub fn physics_shape(address: u64) -> Result<PhysicsShape> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        let spatial = objects
            .character_spatial(target)?
            .ok_or(Error::NativeOperationRejected("HUD spatial module absent"))?;
        let physics = objects
            .character_physics(target)?
            .ok_or(Error::NativeOperationRejected("HUD physics module absent"))?;
        if spatial.module != physics.module
            || objects.character_spatial(target)? != Some(spatial)
            || objects.character_physics(target)? != Some(physics)
        {
            return Err(Error::NativeOperationRejected(
                "HUD physics shape superseded",
            ));
        }
        let [height, radius] = physics.character_collision_bits.map(f32::from_bits);
        Ok(PhysicsShape {
            position_bits: spatial.position_bits,
            height,
            radius,
        })
    })
}

pub fn model_bounds(address: u64) -> Result<Option<[[u32; 3]; 2]>> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        Ok(objects
            .character_model_bounds(target)?
            .map(|value| [value.minimum_bits, value.maximum_bits]))
    })
}
pub fn module_address(address: u64, kind: CharacterModule) -> Result<u64> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_module(target, kind)?
            .map(|module| module.address)
            .ok_or(Error::NativeOperationRejected(
                "HUD selected character module absent",
            ))
    })
}

/// Copy the complete bounded effect-ID sequence; no pointer into the native list escapes.
pub fn effects(address: u64) -> Result<Vec<i32>> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_effects(target, 4096)?
            .map(|value| {
                value
                    .entries
                    .into_iter()
                    .map(|entry| entry.native_id)
                    .collect()
            })
            .ok_or(Error::NativeOperationRejected("HUD effect list absent"))
    })
}
pub fn posture_current(address: u64) -> Result<f32> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_posture(
                target,
                shikifsrs_elden_ring::character_posture::PostureKind::SuperArmor,
            )?
            .map(|value| f32::from_bits(value.current_bits))
            .ok_or(Error::NativeOperationRejected("HUD current posture absent"))
    })
}

pub fn resistance(address: u64) -> Result<[i32; 7]> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_resistance(target)?
            .map(|value| value.words)
            .ok_or(Error::NativeOperationRejected(
                "HUD resistance module absent",
            ))
    })
}

pub fn scalar(
    address: u64,
    field: shikifsrs_elden_ring::character_scalars::CharacterScalar,
) -> Result<shikifsrs_elden_ring::character_scalars::CharacterScalarSnapshot> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_scalar(
                shikifsrs_elden_ring::character_scalars::CharacterScalarKey {
                    character: target,
                    field,
                },
            )?
            .ok_or(Error::NativeOperationRejected("HUD selected scalar absent"))
    })
}

/// # Safety
/// Retain this exact live initialized scalar storage, complete character/module graph,
/// exclusive writable callback phase and full identity through the conditional store.
pub unsafe fn replace_scalar(
    source: shikifsrs_elden_ring::character_scalars::CharacterScalarSnapshot,
    value: f32,
) -> Result<()> {
    mutate(source.key.character.address, |access, record| {
        // SAFETY: The original source is compared again; actual outcome and postflight are retained.
        unsafe { access.scalar_from(record, source, value) }
    })
}

struct MutationSession {
    epoch: u64,
    patched: bool,
    access: crate::character_runtime::NativePresentation,
    records: std::collections::VecDeque<crate::character_runtime::Record>,
    blocked: bool,
}
#[derive(Default)]
struct Mutations {
    current: Option<MutationSession>,
    old: Vec<MutationSession>,
}
thread_local! { static MUTATIONS: std::cell::RefCell<Mutations> = std::cell::RefCell::new(Mutations::default()); }
fn mutate(
    address: u64,
    run: impl FnOnce(
        &crate::character_runtime::NativePresentation,
        &mut crate::character_runtime::Record,
    ) -> Result<()>,
) -> Result<()> {
    let target = native_vitals::identity(address)?;
    MUTATIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD character mutation"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected(
                "revoked HUD character mutation",
            ));
        }
        if slot
            .current
            .as_ref()
            .is_some_and(|session| session.epoch != epoch || session.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "HUD character mutation quarantine full",
                ));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Actual host pins this DLL. Calling public unsafe mutations additionally
            // supplies the exclusive full native graph, writable phase and transitive lifetimes.
            let access = unsafe {
                crate::character_runtime::NativePresentation::bind(
                    bytes, base, epoch, 4096, patches,
                )
            }?;
            slot.current = Some(MutationSession {
                epoch,
                patched,
                access,
                records: std::collections::VecDeque::new(),
                blocked: false,
            });
        }
        let session = slot.current.as_mut().unwrap();
        if session.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD character mutation blocked; actual records retained",
            ));
        }
        if session.records.len() == 64 {
            session.records.pop_front();
        }
        // Publish this identity-only record before any native call or conditional store.
        // Resource requests cannot use it: no registration is fabricated.
        session
            .records
            .push_back(crate::character_runtime::Record::for_identity(target));
        let record = session.records.back_mut().unwrap();
        // Remain blocked even if a Rust unwind interrupts native dispatch or postflight.
        // Only an explicitly returned successful operation may reopen this session.
        session.blocked = true;
        let result = run(&session.access, record);
        if result.is_ok() {
            session.blocked = false;
        }
        if let Err(error) = &result {
            record.failure = Some(error.clone());
            session.blocked = true;
            eprintln!(
                "ShikiFSRS HUD character mutation retained: {error}; {}",
                record.summary()
            );
        }
        result
    })
}
/// # Safety
/// Retain the exclusive event/input/animation graph and exact native callback phase through postflight.
pub unsafe fn request_animation(address: u64, identifier: i32) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward the caller's complete native animation request contract.
        unsafe { access.animation(record, identifier, false) }
    })
}
/// # Safety
/// Retain the full effect/PARAM/allocator/unlink graph and exclusive native phase.
pub unsafe fn apply_effect(address: u64, identifier: i32, force: bool) -> Result<()> {
    let id = shikifsrs_elden_ring::effects::EffectId::new(identifier)?;
    mutate(address, |access, record| {
        // SAFETY: Actual native call and uncertain return are recorded before returning.
        unsafe { access.apply_effect(record, id, force) }
    })
}
/// # Safety
/// Retain the exact live initialized team byte and native character identity under exclusive mutation.
pub unsafe fn set_team(address: u64, team: u8) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward the caller's exact original selected-byte ownership contract.
        unsafe { access.set_team(record, team) }
    })
}

/// # Safety
/// Retain the complete unlink/PARAM/effect allocator graph and exclusive native phase.
pub unsafe fn remove_effect(address: u64, identifier: i32) -> Result<()> {
    let id = shikifsrs_elden_ring::effects::EffectId::new(identifier)?;
    mutate(address, |access, record| {
        // SAFETY: The real native removal attempt and uncertain result remain recorded.
        unsafe { access.remove_effect(record, id) }
    })
}

/// The consumer publishes diagnostics for its original void effect operations.
/// The session retains the real failed attempt and blocks subsequent native mutations.
pub fn report_effect_result(result: Result<()>) {
    if let Err(error) = result {
        eprintln!("ShikiFSRS HUD effect operation failed or uncertain; record retained: {error}");
    }
}

// Consumer request tokens, not a native bitfield layout.
pub const DEBUG_DISABLED_HIT: u32 = 1 << 3;
pub const DEBUG_DISABLED_SECONDARY_ACTIONS: u32 = 1 << 4;
pub const DEBUG_DISABLED_MOVEMENT: u32 = 1 << 5;
pub const DEBUG_DISABLED_UPDATES: u32 = 1 << 8;
pub const DEBUG_FORCE_UNLOADED: u32 = 1 << 10;
pub const DEBUG_FORCE_LOADED: u32 = 1 << 11;
pub const DEBUG_CHARACTER_DISABLED: u32 = 1 << 12;
const DEBUG_REQUESTS: u32 = DEBUG_DISABLED_HIT
    | DEBUG_DISABLED_SECONDARY_ACTIONS
    | DEBUG_DISABLED_MOVEMENT
    | DEBUG_DISABLED_UPDATES
    | DEBUG_FORCE_UNLOADED
    | DEBUG_FORCE_LOADED
    | DEBUG_CHARACTER_DISABLED;
fn debug_plan(set: u32, clear: u32) -> Result<[(u8, u8); 2]> {
    if (set | clear) & !DEBUG_REQUESTS != 0 {
        return Err(Error::InvalidProfile("unknown HUD debug request token"));
    }
    let mut plan = [(0, 0); 2];
    for (token, byte, mask) in [
        (DEBUG_DISABLED_HIT, 0, 8),
        (DEBUG_DISABLED_SECONDARY_ACTIONS, 0, 0x10),
        (DEBUG_DISABLED_MOVEMENT, 0, 0x20),
        (DEBUG_DISABLED_UPDATES, 1, 1),
        (DEBUG_FORCE_UNLOADED, 1, 4),
        (DEBUG_FORCE_LOADED, 1, 8),
        (DEBUG_CHARACTER_DISABLED, 1, 0x10),
    ] {
        if (set | clear) & token != 0 {
            plan[byte].0 |= mask;
        }
        if set & !clear & token != 0 {
            plan[byte].1 |= mask;
        }
    }
    Ok(plan)
}
/// Returns selected raw per-character requests; no engine readiness or visibility claim.
pub fn debug_requests(address: u64) -> Result<(bool, bool)> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        let flags = objects
            .character_flags(target)?
            .ok_or(Error::NativeOperationRejected("HUD debug actor absent"))?;
        let high = flags.value(shikifsrs_elden_ring::character_flags::CharacterFlagByte::DebugHigh);
        Ok((high & 4 != 0, high & 0x10 != 0))
    })
}
/// # Safety
/// Own the selected initialized native bytes and full character graph in an exclusive phase.
pub unsafe fn update_debug_requests(address: u64, set: u32, clear: u32) -> Result<()> {
    let plan = debug_plan(set, clear)?;
    mutate(address, |access, record| {
        for (field, (mask, bits)) in [
            shikifsrs_elden_ring::character_flags::CharacterFlagByte::DebugLow,
            shikifsrs_elden_ring::character_flags::CharacterFlagByte::DebugHigh,
        ]
        .into_iter()
        .zip(plan)
        {
            if mask != 0 {
                // SAFETY: Exact selected bytes; every actual prefix stays in this record.
                unsafe { access.raw_flag(record, field, mask, bits) }?;
            }
        }
        Ok(())
    })
}
/// # Safety
/// Own the exact initialized effect-source word and full character graph in an exclusive phase.
pub unsafe fn clear_effect_source(address: u64) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward the caller contract, retaining the actual conditional store.
        unsafe { access.clear_effect_source(record) }
    })
}
/// # Safety
/// Own the selected status bytes and full graph. These raw marks do not prove engine completion.
pub unsafe fn set_presentation_marks(
    address: u64,
    death: bool,
    model: bool,
    bit1c8_0: bool,
) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward the exact writable-byte contract; retain all successful prefixes.
        unsafe {
            access.death_mark(record, death)?;
            access.set_model(record, model)?;
            access.raw_flag(
                record,
                shikifsrs_elden_ring::character_flags::CharacterFlagByte::Status2,
                1,
                u8::from(bit1c8_0),
            )
        }
    })
}
/// # Safety
/// Own the selected initialized policy bit3 and full native graph in an exclusive phase.
pub unsafe fn set_policy_bit3(address: u64, enabled: bool) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Independently selected NPC initialization policy bit; separate from fallback bit0.
        unsafe { access.raw_policy(record, 8, if enabled { 8 } else { 0 }) }
    })
}
/// # Safety
/// Own every selected opacity word under the full exclusive graph contract through postflight.
pub unsafe fn opacity(address: u64, visible: bool) -> Result<()> {
    use shikifsrs_elden_ring::character_scalars::CharacterScalar::*;
    let fields: &[shikifsrs_elden_ring::character_scalars::CharacterScalar] = if visible {
        &[
            OpacityFactor0,
            OpacityWord23c,
            OpacityFactor1,
            OpacityControl244,
            OpacityFactor2,
            BaseOpacity,
            BaseOpacityControl250,
        ]
    } else {
        &[
            BaseOpacity,
            BaseOpacityControl250,
            OpacityFactor1,
            OpacityControl244,
        ]
    };
    mutate(address, |access, record| {
        for &field in fields {
            // SAFETY: Forward the exact selected-word contract and retain each actual write.
            unsafe { access.scalar(record, field, if visible { 1.0 } else { 0.0 }) }?;
        }
        Ok(())
    })
}

/// Selected cached model-matrix row, distinct from physics position and native getter output.
pub fn model_translation(address: u64) -> Result<[u32; 4]> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_model_translation(target)?
            .map(|value| value.translation_bits)
            .ok_or(Error::NativeOperationRejected(
                "HUD controller model translation absent",
            ))
    })
}

#[derive(Debug, Clone, Copy)]
pub struct MarkerGeometry {
    pub model_base: [f32; 4],
    pub frontend_displacement: [f32; 3],
    pub frontend_anchor: [f32; 3],
}
/// Independent frontend model-up/height equation. This is an experimental HUD
/// anchor policy, not an assertion about the old library's named tag cache.
pub fn marker_geometry(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<MarkerGeometry> {
    sdk_host::with_objects(|objects, epoch| {
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected("HUD marker epoch changed"));
        }
        let rows = objects
            .character_model_anchor_rows(target)?
            .ok_or(Error::NativeOperationRejected("HUD model anchor absent"))?;
        let physics = objects
            .character_physics(target)?
            .ok_or(Error::NativeOperationRejected("HUD marker physics absent"))?;
        if objects.character_model_anchor_rows(target)? != Some(rows)
            || objects.character_physics(target)? != Some(physics)
        {
            return Err(Error::NativeOperationRejected("HUD marker source changed"));
        }
        let height = f32::from_bits(physics.map_collision_bits[0]);
        Ok(MarkerGeometry {
            model_base: rows.translation_bits.map(f32::from_bits),
            frontend_displacement: rows.frontend_displacement_xyz(height)?,
            frontend_anchor: rows.lifted_xyz(height)?,
        })
    })
}

/// Actual raw D0 owner cache, translated by native3f7a20 and consumed by one
/// manipulator branch3d9520. Neither API nor diagnostics claim lock-target semantics.
pub fn position_cache_d0(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<[f32; 4]> {
    sdk_host::with_objects(|objects, epoch| {
        if target.epoch != epoch {
            return Err(Error::NativeOperationRejected("HUD cache epoch changed"));
        }
        objects
            .character_placement(target)?
            .map(|copy| copy.positiond0_bits.map(f32::from_bits))
            .ok_or(Error::NativeOperationRejected("HUD placement cache absent"))
    })
}

pub fn axis_scale(
    target: shikifsrs_elden_ring::character_sets::CharacterIdentity,
) -> Result<[f32; 3]> {
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD axis-scale epoch changed",
            ));
        }
        objects
            .character_controller(target)?
            .map(|copy| copy.axis_scale())
            .ok_or(Error::NativeOperationRejected(
                "HUD controller scale absent",
            ))
    })
}

pub fn load_requests(
    address: u64,
) -> Result<shikifsrs_elden_ring::character_load_requests::CharacterLoadRequestObservation> {
    sdk_host::with_objects(|objects, epoch| {
        let target =
            objects
                .character_identity(address, epoch)?
                .ok_or(Error::NativeOperationRejected(
                    "HUD loading character absent",
                ))?;
        objects
            .character_load_requests(target)?
            .ok_or(Error::NativeOperationRejected(
                "HUD loading character changed",
            ))
    })
}
/// # Safety
/// Retain the initialized writable request word and full exclusive character graph/phase.
pub unsafe fn set_loading_requests(address: u64, mask: u32, bits: u32) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward the public original-word and exclusive full graph contract.
        unsafe { access.raw_loading(record, mask, bits) }
    })
}

pub fn flags(
    address: u64,
) -> Result<shikifsrs_elden_ring::character_flags::CharacterFlagsObservation> {
    let target = native_vitals::identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_flags(target)?
            .ok_or(Error::NativeOperationRejected("HUD character flags absent"))
    })
}
/// # Safety
/// Own the exact initialized result byte and full exclusive character graph through postflight.
/// Direct storage preserves the HUD policy; it does not execute the native group query.
pub unsafe fn set_group_mask_result(address: u64, enabled: bool) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Caller retains the full graph and original selected writable status byte.
        unsafe {
            access.raw_flag(
                record,
                shikifsrs_elden_ring::character_flags::CharacterFlagByte::Status0,
                8,
                if enabled { 8 } else { 0 },
            )
        }
    })
}
/// # Safety
/// Retain initialized exact writable action word and exclusive complete character/module graph.
pub unsafe fn floating_gauge(address: u64, disabled: bool) -> Result<()> {
    mutate(address, |access, record| {
        // SAFETY: Forward public full graph, original extent and exclusive native phase contract.
        unsafe { access.floating_gauge(record, disabled) }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_request_translation_preserves_clear_priority_and_other_native_bits() {
        let tokens = [
            DEBUG_DISABLED_HIT,
            DEBUG_DISABLED_SECONDARY_ACTIONS,
            DEBUG_DISABLED_MOVEMENT,
            DEBUG_DISABLED_UPDATES,
            DEBUG_FORCE_UNLOADED,
            DEBUG_FORCE_LOADED,
            DEBUG_CHARACTER_DISABLED,
        ];
        for selected in 0u32..128 {
            for cleared in 0u32..128 {
                let set = (0..7)
                    .filter(|i| selected & (1 << i) != 0)
                    .fold(0, |value, i| value | tokens[i]);
                let clear = (0..7)
                    .filter(|i| cleared & (1 << i) != 0)
                    .fold(0, |value, i| value | tokens[i]);
                let [(low_mask, low_bits), (high_mask, high_bits)] =
                    debug_plan(set, clear).unwrap();
                assert_eq!(low_mask & !0x38, 0);
                assert_eq!(high_mask & !0x1d, 0);
                for original in [0u8, 0xff, 0xa5, 0x5a] {
                    let low = (original & !low_mask) | low_bits;
                    let high = (original & !high_mask) | high_bits;
                    for (index, word, bit) in [
                        (0, low, 8),
                        (1, low, 0x10),
                        (2, low, 0x20),
                        (3, high, 1),
                        (4, high, 4),
                        (5, high, 8),
                        (6, high, 0x10),
                    ] {
                        let expected = if cleared & (1 << index) != 0 {
                            false
                        } else if selected & (1 << index) != 0 {
                            true
                        } else {
                            original & bit != 0
                        };
                        assert_eq!(word & bit != 0, expected);
                    }
                    assert_eq!(low & !low_mask, original & !low_mask);
                    assert_eq!(high & !high_mask, original & !high_mask);
                }
            }
        }
        assert!(debug_plan(1 << 31, 0).is_err());
    }
}
