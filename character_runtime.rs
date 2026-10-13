//! Actual SDK operations for resident presentation. Each partial operation stays recorded.
use shikifsrs_elden_ring::{
    character_animation::{AnimationOperation, NativeAnimationCall},
    character_flags::{CharacterFlagByte, CharacterFlagWrite},
    character_handles::EffectSourceClear,
    character_resources::{
        CharacterBackreadRequest, CharacterBackreadWrite, CharacterResourcesObservation,
    },
    character_scalars::{
        CharacterReplaceOutcome, CharacterScalar, CharacterScalarAccess, CharacterScalarKey,
        CharacterScalarSnapshot,
    },
    character_sets::{
        CharacterIdentity, CharacterRegistrationObservation, WorldCollectionSelection,
    },
    character_spatial::{NativeCharacterPlacementCall, NativePoseInput},
    character_state_words::{CharacterStateWordWrite, CurrentCharacterStateWords},
    character_units::{CharacterUnitReadPlan, UnitComponent},
    character_vitals::{CharacterVital, CharacterVitalWrite},
    controller_policy::ControllerPolicyWrite,
    current_character_animation::CurrentCharacterAnimation,
    current_character_flags::CurrentCharacterFlags,
    current_character_resources::CurrentCharacterResources,
    current_character_scalars::CurrentCharacterScalarAccess,
    current_character_spatial::CurrentCharacterSpatial,
    current_character_units::CurrentCharacterUnits,
    current_character_vitals::CurrentCharacterVitals,
    current_control_links::CurrentControlLinks,
    current_controller_policy::CurrentControllerPolicy,
    current_effects::CurrentEffectEngine,
    effects::{EffectApplication, EffectEngine, EffectId},
};
use shikifsrs_runtime::{Error, Result};
use std::sync::Arc;

pub enum Operation {
    Backread(CharacterBackreadWrite),
    Flags(CharacterFlagWrite),
    #[allow(dead_code)] // HUD-only eight-byte input modifier operation.
    InputModifiers(shikifsrs_elden_ring::character_aim::CharacterInputModifierWrite),
    State(CharacterStateWordWrite),
    #[allow(dead_code)] // HUD loading path; unused by the separate attack-summon member.
    Load(shikifsrs_elden_ring::character_load_requests::CharacterLoadRequestWrite),
    Scalar {
        source: CharacterScalarSnapshot,
        bits: u32,
        outcome: CharacterReplaceOutcome,
        postflight: Result<CharacterScalarSnapshot>,
    },
    Policy(ControllerPolicyWrite),
    Vitals(CharacterVitalWrite),
    Animation(NativeAnimationCall),
    Euler(shikifsrs_elden_ring::current_character_spatial::NativeEulerCall),
    Placement(Box<NativeCharacterPlacementCall>),
    #[allow(dead_code)] // HUD camera movement uses narrow primary-position publication.
    PositionRequest(shikifsrs_elden_ring::physics_position_requests::PhysicsPositionRequestWrite),
    EffectSource(EffectSourceClear),
    ApplyEffect {
        target: CharacterIdentity,
        id: EffectId,
        returned: Option<Result<EffectApplication>>,
    },
    RemoveEffect {
        target: CharacterIdentity,
        id: EffectId,
        returned: Option<Result<bool>>,
    },
}
/// Store this record with its actor before any operation, including one whose native
/// return is uncertain. Records grant neither native ownership nor a destructor barrier.
pub struct Record {
    pub actor: Option<CharacterRegistrationObservation>,
    pub target: CharacterIdentity,
    pub operations: Vec<Operation>,
    pub failure: Option<Error>,
}
impl Record {
    /// Diagnostic reads of the retained actual receipts, only used after a failure.
    pub fn summary(&self) -> String {
        self.operations
            .iter()
            .map(|operation| match operation {
                Operation::Backread(v) => format!("backread={v:?}"),
                Operation::State(v) => format!("state={v:?}"),
                Operation::Load(v) => format!("loading={v:?}"),
                Operation::Scalar {
                    source,
                    bits,
                    outcome,
                    postflight,
                } => format!("scalar={source:?}/{bits:x}/{outcome:?}/{postflight:?}"),
                Operation::Flags(v) => format!("flags={v:?}"),
                Operation::InputModifiers(v) => format!("input-modifiers={v:?}"),
                Operation::Policy(v) => format!("policy={v:?}"),
                Operation::Vitals(v) => format!("vitals={v:?}"),
                Operation::Euler(v) => format!("euler={v:?}"),
                Operation::Animation(v) => format!("animation={v:?}"),
                Operation::Placement(v) => format!("placement={v:?}"),
                Operation::PositionRequest(v) => format!("position-request={v:?}"),
                Operation::EffectSource(v) => format!("effect-source={v:?}"),
                Operation::ApplyEffect {
                    target,
                    id,
                    returned,
                } => format!("apply target={target:?} id={id:?} result={returned:?}"),
                Operation::RemoveEffect {
                    target,
                    id,
                    returned,
                } => format!("remove target={target:?} id={id:?} result={returned:?}"),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
    pub fn new(actor: CharacterRegistrationObservation) -> Self {
        Self {
            actor: Some(actor),
            target: actor.character,
            operations: Vec::with_capacity(32),
            failure: None,
        }
    }
    /// A copied identity for ordinary mutations, without inventing registration or ownership.
    #[allow(dead_code)] // Ordinary HUD mutations use identities; summon-only builds require registration.
    pub fn for_identity(target: CharacterIdentity) -> Self {
        Self {
            actor: None,
            target,
            operations: Vec::with_capacity(32),
            failure: None,
        }
    }
    fn check(&self) -> Result<()> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.operations.iter().any(|operation| {
            matches!(
                operation,
                Operation::ApplyEffect { returned: None, .. }
                    | Operation::RemoveEffect { returned: None, .. }
            )
        }) {
            return Err(Error::NativeOperationRejected(
                "unresolved native effect attempt retained",
            ));
        }
        if self.operations.len() >= 32 {
            return Err(Error::InvalidProfile("presentation record full"));
        }
        Ok(())
    }
    fn commit(&mut self, operation: Operation, completion: Result<()>) -> Result<()> {
        self.operations.push(operation);
        if let Err(error) = &completion {
            self.failure = Some(error.clone());
        }
        completion
    }
}
pub struct NativePresentation {
    units: CurrentCharacterUnits,
    pub resources: CurrentCharacterResources,
    pub flags: CurrentCharacterFlags,
    state: CurrentCharacterStateWords,
    #[allow(dead_code)]
    // HUD-only gauge suppression; shared attack member has narrower operations.
    input_modifiers: shikifsrs_elden_ring::character_aim::CurrentCharacterInputModifiers,
    #[allow(dead_code)] // HUD loading path; unused by the separate attack-summon member.
    loading: shikifsrs_elden_ring::character_load_requests::CurrentCharacterLoadRequests,
    pub opacity: CurrentCharacterScalarAccess,
    pub vitals: CurrentCharacterVitals,
    pub animation: CurrentCharacterAnimation,
    pub spatial: CurrentCharacterSpatial,
    pub effects: CurrentEffectEngine,
    pub policy: CurrentControllerPolicy,
    pub links: CurrentControlLinks,
    pub set_limit: usize,
}
impl NativePresentation {
    pub fn has_effect(&self, actor: CharacterIdentity, id: EffectId) -> Result<bool> {
        let unit = self
            .units
            .unit(actor)?
            .ok_or(Error::NativeOperationRejected("summon identity absent"))?;
        let UnitComponent::Present(effects) = unit.effects else {
            return Err(Error::NativeOperationRejected("summon effect chain absent"));
        };
        Ok(effects
            .entries
            .iter()
            .any(|node| node.native_id == id.value()))
    }
    /// # Safety
    /// Host independently establishes every component's full graph, writable phase,
    /// heap/resource/callback lifetime and module residency contracts. Thread-local
    /// bindings and visible identity checks do not acquire synchronization or a lease.
    pub unsafe fn bind(
        bytes: Arc<[u8]>,
        base: u64,
        epoch: u64,
        set_limit: usize,
        patches: Option<Arc<shikifsrs_runtime::code_patches::OwnedCodePatches>>,
    ) -> Result<Self> {
        if !(1..=65536).contains(&set_limit) {
            return Err(Error::InvalidProfile("invalid presentation set limit"));
        }
        // SAFETY: The caller supplies all eight component binding contracts on this actual thread.
        unsafe {
            Ok(Self {
                units: CurrentCharacterUnits::bind(
                    Arc::clone(&bytes),
                    base,
                    patches.clone(),
                    CharacterUnitReadPlan {
                        effects_limit: Some(4096),
                        ..Default::default()
                    },
                )?,
                resources: CurrentCharacterResources::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                flags: CurrentCharacterFlags::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                loading: shikifsrs_elden_ring::character_load_requests::CurrentCharacterLoadRequests::bind(
                    Arc::clone(&bytes), base, epoch, patches.clone(),
                )?,
                input_modifiers: shikifsrs_elden_ring::character_aim::CurrentCharacterInputModifiers::bind(
                    Arc::clone(&bytes),base,epoch,patches.clone(),
                )?,
                state: CurrentCharacterStateWords::bind_with_patches(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                opacity: CurrentCharacterScalarAccess::bind_with_patches(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                vitals: CurrentCharacterVitals::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                animation: CurrentCharacterAnimation::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                spatial: CurrentCharacterSpatial::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                policy: CurrentControllerPolicy::bind(
                    Arc::clone(&bytes),
                    base,
                    epoch,
                    patches.clone(),
                )?,
                links: CurrentControlLinks::bind(Arc::clone(&bytes), base, epoch, patches.clone())?,
                effects: CurrentEffectEngine::bind_with_patches(bytes, base, 4096, epoch, patches)?,
                set_limit,
            })
        }
    }
    pub fn resource_marks(
        &self,
        actor: CharacterRegistrationObservation,
    ) -> Result<Option<CharacterResourcesObservation>> {
        let value = self.resources.observe(
            actor.character,
            WorldCollectionSelection::Creation,
            self.set_limit,
        )?;
        if value.is_some_and(|value| value.member.registration != actor) {
            return Err(Error::NativeOperationRejected(
                "presentation actor registration superseded",
            ));
        }
        Ok(value)
    }
    /// # Safety
    /// Same full native graph/exclusive host phase contract; record is retained with actor.
    pub unsafe fn request_present(&self, record: &mut Record) -> Result<()> {
        record.check()?;
        let resource = self
            .resource_marks(record.actor.ok_or(Error::NativeOperationRejected(
                "registered creation actor required",
            ))?)?
            .ok_or(Error::NativeOperationRejected(
                "presentation resources absent",
            ))?;
        // SAFETY: Caller exclusively owns this registered actor's request byte, not a primary.
        let receipt = unsafe {
            self.resources.request(
                resource.member,
                CharacterBackreadRequest::Present,
                self.set_limit,
            )
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Backread(receipt), completion)
    }
    /// # Safety
    /// Same pinned exclusive actor/full graph contract. Stored team is not full hostility.
    pub unsafe fn set_team(&self, record: &mut Record, team: u8) -> Result<()> {
        record.check()?;
        let before = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("team actor absent"))?;
        // SAFETY: Caller owns exactly the selected initialized byte in this native phase.
        let receipt = unsafe {
            self.flags
                .compare_mask(before, CharacterFlagByte::StoredTeam, u8::MAX, team)
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
    /// # Safety
    /// Same full writable graph/phase contract. This model request is not an activation flag.
    pub unsafe fn set_model(&self, record: &mut Record, enabled: bool) -> Result<()> {
        record.check()?;
        let before = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("model actor absent"))?;
        // SAFETY: Caller owns this exact actor's selected request byte.
        let receipt = unsafe { self.flags.set_model_enable_requested(before, enabled) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
    /// # Safety
    /// Exact pinned actor and its original initialized flag byte, exclusive native phase.
    pub unsafe fn set_hit_suppressed(&self, record: &mut Record, suppressed: bool) -> Result<()> {
        record.check()?;
        let before = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("hit actor absent"))?;
        // SAFETY: Same exclusive original selected byte contract.
        let receipt = unsafe { self.flags.set_hit_dispatch_suppressed(before, suppressed) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
    /// # Safety
    /// Exact pinned controller/modifier owner and selected word, exclusive native phase.
    pub unsafe fn set_lock_suppressed(&self, record: &mut Record, suppressed: bool) -> Result<()> {
        record.check()?;
        let before = self
            .policy
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("lock policy absent"))?;
        // SAFETY: Same full controller/modifier graph and exclusive word ownership.
        let receipt = unsafe { self.policy.set_lock_target_suppressed(before, suppressed) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Policy(receipt), completion)
    }
    /// # Safety
    /// Exact pinned controller/modifier and status bytes, exclusive permitted phase. Clearing
    /// these observed marks alone is not revival; reset, HP, resources and presentation are separate.
    pub unsafe fn clear_effect_source(&self, record: &mut Record) -> Result<()> {
        record.check()?;
        let source = self
            .links
            .observe_effect_source(record.target)?
            .ok_or(Error::NativeOperationRejected("effect source actor absent"))?;
        // SAFETY: Host owns the exact initialized source handle word through postflight.
        let receipt = unsafe { self.links.clear_effect_source(source) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::EffectSource(receipt), completion)
    }
    /// # Safety
    /// Same complete writable graph and exclusive native phase; retain every actual prefix.
    pub unsafe fn clear_dead_marks(&self, record: &mut Record) -> Result<()> {
        // SAFETY: Forward the caller contract, retaining the actual source clear.
        unsafe { self.clear_effect_source(record) }?;
        record.check()?;
        let policy = self
            .policy
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("death policy absent"))?;
        // SAFETY: Caller owns the initialized native event-death mark word.
        let receipt = unsafe { self.policy.set_event_dead_marked(policy, false) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Policy(receipt), completion)?;
        record.check()?;
        let before = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("death actor absent"))?;
        // SAFETY: Native3ee730/3f9140 set death-path Status1 bit7; reset3f59c0
        // clears it. Input request bit1 is separate and is preserved.
        let receipt = unsafe { self.flags.set_native_death_marked(before, false) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
    /// # Safety
    /// Same live initialized status byte and exclusive actor graph/phase.
    pub unsafe fn enable_updates(&self, record: &mut Record, enabled: bool) -> Result<()> {
        record.check()?;
        let before = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("update actor absent"))?;
        // SAFETY: Same exact native per-character disable-update request.
        let receipt = unsafe { self.flags.set_debug_update_disabled(before, !enabled) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }

    /// # Safety
    /// Same pinned actor/PARAM/vitals graph in exclusive writable phase. Restoring HP alone
    /// does not clear corpse/event state or make a hidden actor active.
    pub unsafe fn restore_hp(&self, record: &mut Record, dying: bool) -> Result<()> {
        record.check()?;
        let before = self
            .vitals
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("vitals actor absent"))?;
        let value = if dying { 0 } else { before.hp.maximum.max(1) };
        // SAFETY: Caller owns this initialized HP word; no implicit clamping elsewhere.
        let receipt = unsafe {
            self.vitals
                .compare_current(before, CharacterVital::Hp, value)
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Vitals(receipt), completion)
    }
    /// # Safety
    /// Same complete event/module/animation resource graph and exclusive allowed native phase.
    pub unsafe fn animation(&self, record: &mut Record, id: i32, reused: bool) -> Result<()> {
        let ids: &[i32] = if reused { &[-1, id] } else { &[id] };
        for &id in ids {
            record.check()?;
            let before = self
                .animation
                .observe(record.target)?
                .ok_or(Error::NativeOperationRejected("animation actor absent"))?;
            // SAFETY: Caller owns this actor's actual event request and initialized native graph.
            let receipt = unsafe {
                self.animation
                    .execute(before, AnimationOperation::Request(id))
            }?;
            let completion = receipt
                .postflight
                .as_ref()
                .map(|_| ())
                .map_err(Clone::clone);
            record.commit(Operation::Animation(receipt), completion)?;
        }
        Ok(())
    }
    /// # Safety
    /// Same complete native animation request graph/exclusive phase. This explicit park
    /// operation differs from config animation -1 (which means no attempt at all).
    pub unsafe fn clear_animation(&self, record: &mut Record) -> Result<()> {
        record.check()?;
        let before = self
            .animation
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("park animation absent"))?;
        // SAFETY: Caller owns actual event request and full native animation graph.
        let receipt = unsafe {
            self.animation
                .execute(before, AnimationOperation::Request(-1))
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Animation(receipt), completion)
    }
    /// # Safety
    /// Complete native setter/motion/controller graph and exclusive permitted phase.
    pub unsafe fn place_pose(&self, record: &mut Record, input: NativePoseInput) -> Result<()> {
        record.check()?;
        let before = self
            .spatial
            .observe_placement(record.target)?
            .ok_or(Error::NativeOperationRejected("placement actor absent"))?;
        // SAFETY: Caller owns all native placement input and transitive live graph.
        let receipt = unsafe { self.spatial.place_character(before, input) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Placement(Box::new(receipt)), completion)
    }

    /// # Safety
    /// Caller pins all selected physics/controller storage and the complete exclusive update graph.
    #[allow(dead_code)] // Shared source also builds without HUD movement consumers.
    pub unsafe fn publish_primary_position(
        &self,
        record: &mut Record,
        position: [u32; 4],
    ) -> Result<()> {
        record.check()?;
        let source = self
            .spatial
            .observe_position_request(record.target)?
            .ok_or(Error::NativeOperationRejected(
                "position request actor absent",
            ))?;
        // SAFETY: Caller owns the pinned graph and selected exclusive writable extents.
        let receipt = unsafe { self.spatial.publish_position_request(source, position) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::PositionRequest(receipt), completion)?;
        record.check()?;
        let source = self
            .policy
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected(
                "position request controller absent",
            ))?;
        // SAFETY: Same caller graph; only controller E8 bit7, read by 3c8db0 and cleared by 3c8fe0.
        let receipt = unsafe {
            self.policy.compare_mask(
                source,
                shikifsrs_elden_ring::controller_policy::ControllerPolicyField::ControllerLow,
                0x80,
                0x80,
            )
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Policy(receipt), completion)
    }

    /// # Safety
    /// Caller pins actor/effect/PARAM/unlink/allocator and complete transitive graph in an
    /// exclusive native phase. Journal publishes the attempt before entry, including panic.
    pub unsafe fn remove_effect(&self, record: &mut Record, id: EffectId) -> Result<()> {
        record.check()?;
        record.operations.push(Operation::RemoveEffect {
            target: record.target,
            id,
            returned: None,
        });
        // SAFETY: Same pinned actual identity and native unlink graph.
        let returned = unsafe { self.effects.remove(record.target, id) };
        let completion = returned.as_ref().map(|_| ()).map_err(Clone::clone);
        let Operation::RemoveEffect {
            returned: stored, ..
        } = record.operations.last_mut().expect("published attempt")
        else {
            unreachable!()
        };
        *stored = Some(returned);
        if let Err(error) = &completion {
            record.failure = Some(error.clone());
        }
        completion
    }
    /// # Safety
    /// Same complete effect application/PARAM/allocator graph/exclusive phase contract.
    pub unsafe fn ensure_effect(&self, record: &mut Record, id: EffectId) -> Result<()> {
        record.check()?;
        let unit = self
            .units
            .unit(record.target)?
            .ok_or(Error::NativeOperationRejected("summon identity absent"))?;
        let UnitComponent::Present(effects) = unit.effects else {
            return Err(Error::NativeOperationRejected("summon effect chain absent"));
        };
        if effects
            .entries
            .iter()
            .any(|node| node.native_id == id.value())
        {
            return Ok(());
        }
        // SAFETY: Same complete graph and retained record contract, with original forced application.
        unsafe { self.apply_effect(record, id, true) }
    }
    /// # Safety
    /// Host retains the full effect/PARAM/allocation graph and exclusive native phase.
    /// The attempted call is published before entry; even an uncertain return stays retained.
    pub unsafe fn apply_effect(
        &self,
        record: &mut Record,
        id: EffectId,
        force: bool,
    ) -> Result<()> {
        record.check()?;
        record.operations.push(Operation::ApplyEffect {
            target: record.target,
            id,
            returned: None,
        });
        // SAFETY: Same retained summon/full graph. Submitted means an attempt, not gameplay acceptance.
        let returned = unsafe { self.effects.apply(record.target, id, force) };
        let completion = returned.as_ref().map(|_| ()).map_err(Clone::clone);
        let Operation::ApplyEffect {
            returned: stored, ..
        } = record.operations.last_mut().expect("published attempt")
        else {
            unreachable!()
        };
        *stored = Some(returned);
        if let Err(error) = &completion {
            record.failure = Some(error.clone());
        }
        completion
    }
}

impl NativePresentation {
    pub unsafe fn raw_flag(
        &self,
        record: &mut Record,
        field: CharacterFlagByte,
        mask: u8,
        bits: u8,
    ) -> Result<()> {
        record.check()?;
        let source = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("attack flag actor absent"))?;
        // SAFETY: Host provides current native phase/full graph/exclusive selected byte.
        let receipt = unsafe { self.flags.compare_mask(source, field, mask, bits) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
    pub unsafe fn activation_request(&self, record: &mut Record, enabled: bool) -> Result<()> {
        // SAFETY: Forward the exact fallback request contract; this is not policy bit3.
        unsafe { self.raw_policy(record, 1, u32::from(enabled)) }
    }
    /// # Safety
    /// Retain exact initialized writable word40 and full exclusive character/module graph.
    #[allow(dead_code)] // HUD uses this operation; attack-summon does not suppress gauges.
    pub unsafe fn floating_gauge(&self, record: &mut Record, disabled: bool) -> Result<()> {
        record.check()?;
        let source = self
            .input_modifiers
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("HUD gauge module absent"))?;
        // SAFETY: Forward exact original eight-byte extent and exclusive full graph contract.
        let receipt = unsafe {
            self.input_modifiers
                .set_floating_gauge_suppressed(source, disabled)
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::InputModifiers(receipt), completion)
    }
    /// # Safety
    /// Retain the exact initialized writable loading request word and full exclusive actor graph.
    #[allow(dead_code)] // Used by HUD; attack-summon retains its original narrower operations.
    pub unsafe fn raw_loading(&self, record: &mut Record, mask: u32, bits: u32) -> Result<()> {
        record.check()?;
        let source = self
            .loading
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("loading actor absent"))?;
        // SAFETY: Forward the caller's selected original writable extent and phase contract.
        let receipt = unsafe { self.loading.compare_mask(source, mask, bits) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Load(receipt), completion)
    }
    pub unsafe fn raw_policy(&self, record: &mut Record, mask: u32, bits: u32) -> Result<()> {
        record.check()?;
        let source = self
            .state
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("attack policy actor absent"))?;
        // SAFETY: Selected native fallback request; not proof of completed activation.
        let receipt = unsafe { self.state.compare_policy_mask(source, mask, bits) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::State(receipt), completion)
    }
    pub unsafe fn scalar(
        &self,
        record: &mut Record,
        field: CharacterScalar,
        value: f32,
    ) -> Result<()> {
        record.check()?;
        let source = self
            .opacity
            .observe(CharacterScalarKey {
                character: record.target,
                field,
            })?
            .ok_or(Error::NativeOperationRejected("attack scalar actor absent"))?;
        // SAFETY: Forward the same selected original storage and exclusive phase.
        unsafe { self.scalar_from(record, source, value) }
    }
    pub unsafe fn scalar_from(
        &self,
        record: &mut Record,
        source: CharacterScalarSnapshot,
        value: f32,
    ) -> Result<()> {
        record.check()?;
        if source.key.character != record.target {
            return Err(Error::NativeOperationRejected(
                "foreign scalar record identity",
            ));
        }
        let bits = value.to_bits();
        // SAFETY: Exact initialized selected field, exclusive callback graph, receipt retained.
        let outcome = unsafe { self.opacity.compare_replace(source, bits) }?;
        let postflight = self.opacity.observe(source.key).and_then(|v| {
            v.filter(|v| v.bits == bits && v.storage == source.storage)
                .ok_or(Error::NativeOperationRejected("attack scalar postflight"))
        });
        let completion = if outcome == CharacterReplaceOutcome::Applied {
            postflight.as_ref().map(|_| ()).map_err(Clone::clone)
        } else {
            Err(Error::NativeOperationRejected("attack scalar conflict"))
        };
        record.commit(
            Operation::Scalar {
                source,
                bits,
                outcome,
                postflight,
            },
            completion,
        )
    }
    pub unsafe fn request_unload(&self, record: &mut Record) -> Result<()> {
        record.check()?;
        let source = self
            .resource_marks(record.actor.ok_or(Error::NativeOperationRejected(
                "registered creation actor required",
            ))?)?
            .ok_or(Error::NativeOperationRejected("attack resources absent"))?;
        // SAFETY: Host retains complete graph for future request consumption; no destructor/free.
        let receipt = unsafe {
            self.resources.request(
                source.member,
                CharacterBackreadRequest::Unload,
                self.set_limit,
            )
        }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Backread(receipt), completion)
    }
    pub unsafe fn death_mark(&self, record: &mut Record, marked: bool) -> Result<()> {
        record.check()?;
        let source = self
            .flags
            .observe(record.target)?
            .ok_or(Error::NativeOperationRejected("attack death actor absent"))?;
        // SAFETY: Independently proved native death-path request bit7, preserving aim bit1.
        let receipt = unsafe { self.flags.set_native_death_marked(source, marked) }?;
        let completion = receipt
            .postflight
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone);
        record.commit(Operation::Flags(receipt), completion)
    }
}

impl Record {
    pub fn retain_euler(
        &mut self,
        receipt: shikifsrs_elden_ring::current_character_spatial::NativeEulerCall,
        completion: Result<()>,
    ) -> Result<()> {
        self.commit(Operation::Euler(receipt), completion)
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;
    fn target() -> CharacterIdentity {
        CharacterIdentity {
            address: 0x1000,
            world_address: 0x2000,
            native_identifier: 0xabcdef1200000001,
            epoch: 7,
        }
    }
    #[test]
    fn ordinary_identity_never_invents_a_registered_actor() {
        let record = Record::for_identity(target());
        assert_eq!(record.target, target());
        assert!(record.actor.is_none());
        assert!(record.check().is_ok());
    }
    #[test]
    fn unresolved_effect_attempt_blocks_further_operations_and_preserves_failed_prefix() {
        let mut record = Record::for_identity(target());
        let id = EffectId::new(10).unwrap();
        record.operations.push(Operation::ApplyEffect {
            target: target(),
            id,
            returned: None,
        });
        assert!(record.check().is_err());
        assert_eq!(record.operations.len(), 1);
        let error = Error::NativeOperationRejected("actual postflight failed");
        record.failure = Some(error.clone());
        assert_eq!(record.check(), Err(error));
        assert!(matches!(
            record.operations[0],
            Operation::ApplyEffect { returned: None, .. }
        ));
    }
}
