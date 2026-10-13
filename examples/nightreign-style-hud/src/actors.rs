//! Rust-owned HUD observations. Never cast these values to an engine object.
use crate::{native_handles::HudHandle, native_world, sdk_host};
use shikifsrs_elden_ring::{
    character_flags::CharacterFlagByte, character_handles::PrimaryPlayerTargetObservation,
    character_parameters::CharacterInitializerWord, character_sets::CharacterIdentity,
    objects::ObjectObservation,
};
use shikifsrs_runtime::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct HudActor {
    pub identity: CharacterIdentity,
    pub handle: HudHandle,
    pub raw_kind: i32,
    pub character_number: u32,
    pub resource_number: i32,
    pub npc_request_id: i32,
    pub team: u8,
    /// Existing consumer attribution policy; this SDK field is the last effect source,
    /// not proof of a damage event, death cause or universally last attacker.
    pub last_effect_source: HudHandle,
}
impl HudActor {
    pub fn observe(identity: CharacterIdentity) -> Result<Self> {
        sdk_host::with_objects(|objects, epoch| {
            if identity.epoch != epoch {
                return Err(Error::NativeOperationRejected("HUD actor epoch changed"));
            }
            let selection = objects
                .character_selection(identity)?
                .ok_or(Error::NativeOperationRejected("HUD actor selection absent"))?;
            let flags = objects
                .character_flags(identity)?
                .ok_or(Error::NativeOperationRejected("HUD actor flags absent"))?;
            let resource = objects
                .character_initializer_word(identity, CharacterInitializerWord::ResourceNumber)?
                .ok_or(Error::NativeOperationRejected(
                    "HUD actor resource number absent",
                ))?;
            let npc = objects.character_npc_request_id(identity)?.ok_or(
                Error::NativeOperationRejected("HUD actor NPC request absent"),
            )?;
            let effect =
                objects
                    .last_effect_source(identity)?
                    .ok_or(Error::NativeOperationRejected(
                        "HUD actor effect source absent",
                    ))?;
            if objects.character_selection(identity)? != Some(selection)
                || objects.character_flags(identity)? != Some(flags)
                || objects.character_initializer_word(
                    identity,
                    CharacterInitializerWord::ResourceNumber,
                )? != Some(resource)
                || objects.character_npc_request_id(identity)? != Some(npc)
                || objects.last_effect_source(identity)? != Some(effect)
            {
                return Err(Error::NativeOperationRejected("HUD actor fields changed"));
            }
            Ok(Self {
                identity,
                handle: HudHandle::from_native(identity.native_identifier),
                raw_kind: selection.raw_kind,
                character_number: selection.raw_character_number,
                resource_number: resource.value,
                npc_request_id: npc.stored_id,
                team: flags.value(CharacterFlagByte::StoredTeam),
                last_effect_source: HudHandle::from_native(effect.handle.0),
            })
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub struct HudPlayer {
    pub actor: HudActor,
    pub locked_target: HudHandle,
}
impl HudPlayer {
    fn observe(selection: PrimaryPlayerTargetObservation) -> Result<Self> {
        let actor = HudActor::observe(selection.player)?;
        let stable = sdk_host::with_objects(|objects, epoch| objects.primary_player_target(epoch))?;
        if stable != Some(selection) {
            return Err(Error::NativeOperationRejected(
                "HUD primary selection changed",
            ));
        }
        Ok(Self {
            actor,
            locked_target: HudHandle::from_native(selection.handle.0),
        })
    }
}
/// Owned observation only. Mutating this Rust value does not mutate any engine object.
pub struct HudWorld {
    pub object: ObjectObservation,
    pub epoch: u64,
    pub player: Option<HudPlayer>,
}
impl HudWorld {
    pub fn observe() -> Result<Self> {
        let (object, epoch, selection) = sdk_host::with_objects(|objects, epoch| {
            let object = objects
                .world()?
                .ok_or(Error::NativeOperationRejected("HUD world absent"))?;
            let selection = objects.primary_player_target(epoch)?;
            if selection.is_some_and(|value| value.player.world_address != object.address)
                || objects.world()? != Some(object)
            {
                return Err(Error::NativeOperationRejected("HUD world scope changed"));
            }
            Ok((object, epoch, selection))
        })?;
        let player = selection.map(HudPlayer::observe).transpose()?;
        if sdk_host::epoch() != epoch || native_world::world()? != Some(object) {
            return Err(Error::NativeOperationRejected("HUD world superseded"));
        }
        Ok(Self {
            object,
            epoch,
            player,
        })
    }
    /// # Safety
    /// Keep the exact full native world/area/block/set/character graph in its permitted
    /// exclusive callback phase. Copied identities do not pin allocations or prevent ABA.
    pub unsafe fn actor(&self, handle: &HudHandle) -> Result<Option<HudActor>> {
        if self.epoch != sdk_host::epoch() || native_world::world()? != Some(self.object) {
            return Err(Error::NativeOperationRejected(
                "HUD actor lookup world changed",
            ));
        }
        // SAFETY: Forward the caller's complete native lookup graph and phase obligations.
        unsafe { native_world::resolve(self.object.address, handle.raw()) }?
            .map(HudActor::observe)
            .transpose()
    }
}

/// Consumer mathematics only; no engine representation or layout contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudPosition(pub f32, pub f32, pub f32, pub f32);
#[cfg(test)]
impl HudPosition {
    /// Owned XYZ fixture for consumer mathematics; never a native representation.
    pub(crate) fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self(x, y, z, 0.0)
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudDelta(pub f32, pub f32, pub f32);

/// Private host adapter; callback graph/phase obligations remain experimental.
/// The returned actor is a copied Rust value and grants no native borrowing lease.
pub fn hud_actor_by_handle(world: &HudWorld, handle: &HudHandle) -> Option<HudActor> {
    // SAFETY: The owning host callback must retain the full graph documented on actor().
    unsafe { world.actor(handle) }
        .map_err(|error| eprintln!("ShikiFSRS HUD actor lookup rejected: {error}"))
        .ok()
        .flatten()
}

impl std::ops::Sub for HudPosition {
    type Output = HudDelta;
    fn sub(self, other: Self) -> HudDelta {
        HudDelta(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }
}
