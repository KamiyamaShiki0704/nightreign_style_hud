//! Thread-confined current-EXE task host. Static/offline evidence grants no phase lease.
use super::{
    AttackSummonConfig, AttackSummonSettings, Cooldowns,
    creation::{self, Attempt, NativeDriver, Queue, Request},
    normalize_horizontal,
    presentation::{NativePresentation, Record},
    sdk_host, settings, summon_position,
};
use shikifsrs_elden_ring::{
    TaskCallback, TaskInvocation, Ww171File,
    character_flags::CharacterFlagByte,
    character_scalars::CharacterScalar,
    character_sets::{CharacterIdentity, CharacterRegistrationObservation},
    character_spatial::NativePoseInput,
    character_units::{CharacterUnitReadPlan, UnitComponent},
    creation_input_preparation::OrdinaryCreationInput,
    current_character_units::CurrentCharacterUnits,
    current_debug_creator::CurrentDebugCreator,
    current_effects::CurrentEffectEngine,
    current_tasks::CurrentTaskEngine,
    debug_creation_fields::OrdinaryCreationRoute,
    effects::{EffectEngine, EffectId},
    task_resident::ResidentCallbackGroupSlot,
};
use shikifsrs_runtime::{Error, Result, memory::CurrentProcess};
use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
use windows::Win32::System::Threading::GetCurrentThreadId;
const LIMIT: usize = 4096;
static TASKS: ResidentCallbackGroupSlot<1> = ResidentCallbackGroupSlot::new();
static THREAD: AtomicU32 = AtomicU32::new(0);
#[derive(Debug, Clone, Copy, PartialEq)]
enum Step {
    Present,
    Unload,
    Team(u8),
    Lock(bool),
    ClearDeath,
    Death,
    Model(bool),
    Activation(bool),
    Updates(bool),
    Hit(bool),
    Debug(CharacterFlagByte, u8, u8),
    Scalar(CharacterScalar, f32),
    Hp(bool),
    ClearAnimation,
    Animation(i32, bool),
    Marker(i32),
    Remove(i32),
    Place([f32; 4]),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Maintain,
    Bind,
    Reuse,
    Park,
    Release,
}
fn plan(
    mode: Mode,
    config: AttackSummonConfig,
    s: &AttackSummonSettings,
    position: Option<[f32; 4]>,
) -> Vec<Step> {
    let mut steps = Vec::new();
    if mode == Mode::Release {
        steps.extend([
            Step::Hp(true),
            Step::Death,
            Step::Model(false),
            Step::Activation(false),
            Step::Updates(false),
            Step::Hit(true),
            Step::Debug(CharacterFlagByte::DebugLow, 0x30, 0x30),
            Step::Debug(CharacterFlagByte::DebugHigh, 0x1c, 0x14),
        ]);
    } else if mode == Mode::Park {
        steps.extend([
            Step::Remove(config.vanish_request_speffect(s)),
            Step::Hp(false),
            Step::ClearAnimation,
            Step::ClearDeath,
            Step::Model(false),
            Step::Activation(true),
            Step::Present,
            Step::Updates(true),
            Step::Debug(CharacterFlagByte::DebugLow, 0x30, 0),
            Step::Debug(CharacterFlagByte::DebugHigh, 0x1c, 8),
            Step::Hit(true),
        ]);
    } else {
        if mode == Mode::Reuse {
            steps.push(Step::Remove(config.vanish_request_speffect(s)));
        }
        steps.extend([
            Step::Marker(config.marker_speffect(s)),
            Step::Team(config.generated_team_type(s)),
            Step::Lock(config.disable_lock_on(s)),
            Step::ClearDeath,
            Step::Present,
            Step::Model(true),
            Step::Activation(true),
            Step::Updates(true),
            Step::Debug(CharacterFlagByte::DebugLow, 0x30, 0),
            Step::Debug(CharacterFlagByte::DebugHigh, 0x1c, 8),
            Step::Hit(false),
        ]);
        for field in [
            CharacterScalar::OpacityFactor0,
            CharacterScalar::OpacityWord23c,
            CharacterScalar::OpacityFactor1,
            CharacterScalar::OpacityControl244,
            CharacterScalar::OpacityFactor2,
            CharacterScalar::BaseOpacity,
            CharacterScalar::BaseOpacityControl250,
        ] {
            steps.push(Step::Scalar(field, 1.0));
        }
        if let Some(position) = position {
            steps.push(Step::Place(position));
        }
        if mode == Mode::Reuse {
            steps.push(Step::Hp(false));
        }
        if mode == Mode::Bind || mode == Mode::Reuse {
            steps.push(Step::Animation(config.animation_id, mode == Mode::Reuse));
        }
    }
    if mode == Mode::Park || mode == Mode::Release {
        for field in [
            CharacterScalar::BaseOpacity,
            CharacterScalar::BaseOpacityControl250,
            CharacterScalar::OpacityFactor1,
            CharacterScalar::OpacityControl244,
        ] {
            steps.push(Step::Scalar(field, 0.0));
        }
    }
    if mode == Mode::Release {
        steps.push(Step::Unload);
    }
    steps
}
struct Player {
    target: CharacterIdentity,
    position: [f32; 4],
    triggers: Vec<usize>,
}
struct Instance {
    config_index: usize,
    actor: CharacterRegistrationObservation,
    expires_at: Instant,
    active: bool,
    needs_binding: bool,
    /// Keep the unique creator ticket for this native actor, including soft-release/parking.
    ticket: creation::Pending<Attempt>,
}
struct TriggerAttempt {
    target: CharacterIdentity,
    id: EffectId,
    returned: Option<Result<bool>>,
}
struct Core {
    epoch: u64,
    units: CurrentCharacterUnits,
    effects: CurrentEffectEngine,
    presentation: NativePresentation,
    driver: NativeDriver,
    queue: Queue<Attempt>,
    instances: Vec<Instance>,
    retained: Vec<Instance>,
    cooldowns: Cooldowns,
    records: VecDeque<Record>,
    triggers: VecDeque<TriggerAttempt>,
    failure: Option<Error>,
}
#[derive(Default)]
struct Sessions {
    current: Option<Core>,
    old: Vec<Core>,
}
thread_local! {static SESSIONS:RefCell<Sessions>=RefCell::new(Sessions::default());}
impl Core {
    fn bind(epoch: u64) -> Result<Self> {
        let (bytes, base) = sdk_host::binding()?;
        // SAFETY: Resident callback thread owns sessions; complete actual native graphs,
        // exclusivity, phase order and epoch lifetimes remain experimental caller obligations.
        unsafe {
            Ok(Self {
                epoch,
                units: CurrentCharacterUnits::bind(
                    bytes.clone(),
                    base,
                    None,
                    CharacterUnitReadPlan {
                        effects_limit: Some(LIMIT),
                        ..Default::default()
                    },
                )?,
                effects: CurrentEffectEngine::bind(bytes.clone(), base, LIMIT, epoch)?,
                presentation: NativePresentation::bind(
                    bytes.clone(),
                    base,
                    epoch,
                    LIMIT,
                    sdk_host::patches(),
                )?,
                driver: NativeDriver {
                    creator: CurrentDebugCreator::bind(bytes, base, epoch, None)?,
                    pending: None,
                    set_limit: LIMIT,
                },
                queue: Queue::new(settings().max_active_summons.clamp(1, 65536))?,
                instances: Vec::new(),
                retained: Vec::new(),
                cooldowns: Cooldowns::default(),
                records: VecDeque::new(),
                triggers: VecDeque::new(),
                failure: None,
            })
        }
    }
    fn execute(&mut self, actor: CharacterRegistrationObservation, steps: Vec<Step>) -> Result<()> {
        if self.records.len() == 64 {
            self.records.pop_front();
        }
        // Publish ownership of this operation record before the first attempt.
        self.records.push_back(Record::new(actor));
        let record = self.records.back_mut().expect("published record");
        let p = &self.presentation;
        let result = (|| -> Result<()> {
            for step in steps {
                // SAFETY: Same exclusive callback full graph contract; all actual partial receipts
                // remain in the published record. No retries, destructors or unloading are implicit.
                unsafe {
                    match step {
                        Step::Present => p.request_present(record)?,
                        Step::Unload => p.request_unload(record)?,
                        Step::Team(v) => p.set_team(record, v)?,
                        Step::Lock(v) => p.set_lock_suppressed(record, v)?,
                        Step::ClearDeath => p.clear_dead_marks(record)?,
                        Step::Death => p.death_mark(record, true)?,
                        Step::Model(v) => p.set_model(record, v)?,
                        Step::Activation(v) => p.activation_request(record, v)?,
                        Step::Updates(v) => p.enable_updates(record, v)?,
                        Step::Hit(v) => p.set_hit_suppressed(record, v)?,
                        Step::Debug(field, mask, bits) => p.raw_flag(record, field, mask, bits)?,
                        Step::Scalar(field, value) => p.scalar(record, field, value)?,
                        Step::Hp(v) => p.restore_hp(record, v)?,
                        Step::ClearAnimation => p.clear_animation(record)?,
                        Step::Animation(id, reused) => p.animation(record, id, reused)?,
                        Step::Marker(id) => p.ensure_effect(record, EffectId::new(id)?)?,
                        Step::Remove(id) => p.remove_effect(record, EffectId::new(id)?)?,
                        Step::Place(position) => {
                            // Preserve the reused actor's current rotation rather than inventing a yaw.
                            let before = p.spatial.euler(actor.character)?;
                            let completion = before.postflight.clone();
                            let angles = before.value.bits;
                            record.retain_euler(before, completion)?;
                            p.place_pose(
                                record,
                                NativePoseInput {
                                    position_bits: position.map(f32::to_bits),
                                    angle_bits: [angles[0], angles[1], angles[2]],
                                },
                            )?;
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = &result {
            record.failure = Some(error.clone());
            eprintln!("attack partial record: {}", record.summary());
        }
        result
    }
    fn player(&self) -> Result<Option<Player>> {
        let observed = sdk_host::with_objects(|objects, epoch| {
            let Some(player) = objects.primary_player()? else {
                return Ok(None);
            };
            let target = objects.character_identity(player.address, epoch)?.ok_or(
                Error::NativeOperationRejected("attack primary identity absent"),
            )?;
            let spatial =
                objects
                    .character_spatial(target)?
                    .ok_or(Error::NativeOperationRejected(
                        "attack primary physics absent",
                    ))?;
            let position = spatial.position_bits.map(f32::from_bits);
            Ok(Some((target, position)))
        })?;
        let Some((target, position)) = observed else {
            return Ok(None);
        };
        let unit = self
            .units
            .unit(target)?
            .ok_or(Error::NativeOperationRejected("attack primary unit absent"))?;
        let UnitComponent::Present(effects) = unit.effects else {
            return Err(Error::NativeOperationRejected(
                "attack primary effect chain absent",
            ));
        };
        let triggers = settings()
            .summons
            .iter()
            .enumerate()
            .filter_map(|(index, c)| {
                effects
                    .entries
                    .iter()
                    .any(|e| e.native_id == c.trigger_speffect)
                    .then_some(index)
            })
            .collect();
        Ok(Some(Player {
            target,
            position,
            triggers,
        }))
    }
    fn consume(&mut self, target: CharacterIdentity, index: usize) -> Result<()> {
        let id = EffectId::new(settings().summons[index].trigger_speffect)?;
        if self.triggers.len() == 64 {
            self.triggers.pop_front();
        }
        self.triggers.push_back(TriggerAttempt {
            target,
            id,
            returned: None,
        });
        // SAFETY: Current primary/effect/PARAM/unlink graph and exclusive phase held by caller.
        let returned = unsafe { self.effects.remove(target, id) };
        let result = returned.as_ref().map(|_| ()).map_err(Clone::clone);
        self.triggers
            .back_mut()
            .expect("published trigger removal")
            .returned = Some(returned);
        result
    }
    fn maintain(&mut self, now: Instant, player_present: bool) -> Result<()> {
        let s = settings();
        if let Some(done) = self
            .queue
            .poll(&mut self.driver, now, s.unbound_timeout())?
        {
            let config_index = done.pending.request.config_index;
            let expires_at = done
                .pending
                .request
                .expires_at
                .ok_or(Error::InvalidProfile("attack lifetime absent"))?;
            let actor = done.registration;
            // Retain unique result before any presentation, including late completed requests.
            self.instances.push(Instance {
                config_index,
                actor,
                expires_at,
                active: true,
                needs_binding: true,
                ticket: done.pending,
            });
        }
        let mut index = 0;
        while index < self.instances.len() {
            let actor = self.instances[index].actor;
            if !self.instances[index].active {
                index += 1;
                continue;
            }
            let resource = self.presentation.resource_marks(actor)?;
            if resource.is_none() {
                let old = self.instances.swap_remove(index);
                self.retained.push(old);
                continue;
            }
            if !resource.expect("observed").native_presentation_marks() {
                index += 1;
                continue;
            }
            let config = s.summons[self.instances[index].config_index];
            if self.instances[index].needs_binding {
                self.execute(actor, plan(Mode::Bind, config, s, None))?;
                self.instances[index].needs_binding = false;
            }
            let flags = self
                .presentation
                .flags
                .observe(actor.character)?
                .ok_or(Error::NativeOperationRejected("attack actor flags absent"))?;
            let vitals = self
                .presentation
                .vitals
                .observe(actor.character)?
                .ok_or(Error::NativeOperationRejected("attack actor vitals absent"))?;
            let disappear = !player_present
                || now >= self.instances[index].expires_at
                || self.instances[index].ticket.timed_out
                || vitals.hp.current <= 0
                || flags.native_death_marked()
                || flags.value(CharacterFlagByte::DebugHigh) & 0x10 != 0
                || self.presentation.has_effect(
                    actor.character,
                    EffectId::new(config.vanish_request_speffect(s))?,
                )?;
            if disappear {
                let mode = if s.reuse_spawned_units && player_present {
                    Mode::Park
                } else {
                    Mode::Release
                };
                self.execute(actor, plan(mode, config, s, None))?;
                if mode == Mode::Park {
                    self.instances[index].active = false;
                    index += 1;
                } else {
                    let old = self.instances.swap_remove(index);
                    self.retained.push(old);
                }
            } else {
                self.execute(actor, plan(Mode::Maintain, config, s, None))?;
                index += 1;
            }
        }
        Ok(())
    }
    fn active_count(&self) -> usize {
        self.instances.iter().filter(|v| v.active).count() + self.queue.pending_count()
    }
    fn activate(&mut self, index: usize, now: Instant, position: [f32; 4]) -> Result<()> {
        let s = settings();
        let config = s.summons[index];
        if s.reuse_spawned_units {
            if self.queue.has_configuration(index) {
                return Ok(());
            }
            if let Some(at) = self.instances.iter().position(|v| v.config_index == index) {
                if self.instances[at].active {
                    return Ok(());
                }
                let actor = self.instances[at].actor;
                if self.presentation.resource_marks(actor)?.is_some() {
                    self.execute(actor, plan(Mode::Reuse, config, s, Some(position)))?;
                    self.instances[at].active = true;
                    self.instances[at].expires_at = now + config.fallback_lifetime(s);
                    return Ok(());
                }
                let old = self.instances.swap_remove(at);
                self.retained.push(old);
            }
        }
        if self.instances.len() + self.retained.len() >= 65536 {
            return Err(Error::NativeOperationRejected(
                "attack retained native actor bound",
            ));
        }
        self.queue.enqueue(Request {
            config_index: index,
            requested_at: now,
            expires_at: Some(now + config.fallback_lifetime(s)),
            input: OrdinaryCreationInput {
                name_units: format!("c{:04}", config.chr_id).encode_utf16().collect(),
                position_bits: position.map(f32::to_bits),
                rotation_bits: [0; 4],
                npc_param_id: config.npc_param_id,
                npc_think_param_id: config.npc_think_param_id,
                event_entity_id: config.event_entity_id as u32,
                talk_id: config.talk_id,
                chara_init_param_id: config.chara_init_param_id,
                animation_id: config.animation_id,
                route: if config.is_player {
                    OrdinaryCreationRoute::Player
                } else {
                    OrdinaryCreationRoute::Enemy
                },
            },
        })
    }
    fn tick(&mut self) -> Result<()> {
        if let Some(error) = self.queue.blocked() {
            return Err(error.clone());
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let now = Instant::now();
        let observed = self.player()?;
        self.maintain(now, observed.is_some())?;
        let Some(Player {
            target,
            position,
            triggers,
        }) = observed
        else {
            self.queue.cancel_unattempted();
            return Ok(());
        };
        // Preserve original consume-before-capacity/cooldown ordering for every matching trigger.
        for &index in &triggers {
            self.consume(target, index)?;
        }
        let (forward, right) = sdk_host::with_objects(|objects, _| {
            let Some(cameras) = objects.cameras()? else {
                return Ok(((0., 0., 1.), (1., 0., 0.)));
            };
            let Some(camera) = cameras.perspectives[0] else {
                return Ok(((0., 0., 1.), (1., 0., 0.)));
            };
            let camera = objects
                .perspective_camera(camera.address)?
                .ok_or(Error::NativeOperationRejected("attack camera replaced"))?;
            let f = camera.matrix_bits[2].map(f32::from_bits);
            let r = camera.matrix_bits[0].map(f32::from_bits);
            Ok((
                normalize_horizontal((f[0], f[1], f[2]), (0., 0., 1.)),
                normalize_horizontal((r[0], r[1], r[2]), (1., 0., 0.)),
            ))
        })?;
        let s = settings();
        for index in triggers {
            if self.active_count() >= s.max_active_summons {
                break;
            }
            if self.cooldowns.accept(index, now, s) {
                self.activate(
                    index,
                    now,
                    summon_position(position, forward, right, s.summons[index], s),
                )?;
            }
        }
        Ok(())
    }
}
fn run() -> Result<()> {
    SESSIONS.with(|cell| {
        let mut sessions = cell
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive attack task"))?;
        let epoch = sdk_host::epoch();
        if sessions.current.as_ref().is_some_and(|v| v.epoch != epoch) {
            if sessions.old.len() >= 64 {
                return Err(Error::NativeOperationRejected(
                    "attack epoch quarantine full",
                ));
            }
            let mut old = sessions.current.take().expect("old session");
            old.queue.quarantine_epoch();
            sessions.old.push(old);
        }
        if sessions.current.is_none() {
            sessions.current = Some(Core::bind(epoch)?);
        }
        let core = sessions.current.as_mut().expect("current session");
        let result = core.tick();
        if let Err(error) = &result {
            if let Some(ticket) = core.queue.active_ticket() {
                eprintln!(
                    "attack creation retained: operations={} failure={:?}",
                    ticket.operations.len(),
                    ticket.failure
                );
            }
            core.failure = Some(error.clone());
            for item in &core.triggers {
                if item.returned.as_ref().is_none_or(|v| v.is_err()) {
                    eprintln!("attack trigger retained: {:?} {:?}", item.target, item.id);
                }
            }
        }
        result
    })
}
pub fn install() -> Result<()> {
    let (bytes, base) = sdk_host::binding()?;
    let file = Ww171File::verify(&bytes)?;
    let phase = file.phase("ChrIns_PostPhysics")?;
    let start = Instant::now();
    let group = loop {
        let tasks = file.bind(&CurrentProcess, base)?;
        if let Some(id) = tasks
            .observe_phase(phase)?
            .and_then(|v| v.native_group_id())
            && tasks.registered_group(id, LIMIT)?.is_some()
        {
            break id;
        }
        if start.elapsed() >= Duration::from_secs(30) {
            return Err(Error::NativeOperationRejected(
                "attack task startup timeout",
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    // SAFETY: Host pinned DLL retains engine and partial registration; native scheduler
    // transitive lifetime/exclusive callback phase remains experimental, no live validation.
    let engine = unsafe { CurrentTaskEngine::bind(bytes, base) }?;
    let callback = TaskCallback::new(move |v: TaskInvocation| {
        // SAFETY: OS thread ID query only; not an engine phase lock.
        let thread = unsafe { GetCurrentThreadId() };
        let same = match THREAD.compare_exchange(0, thread, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => true,
            Err(old) => old == thread,
        };
        if !same || v.group_id != group {
            eprintln!("attack phase/thread rejected");
            return;
        }
        if let Err(error) = run() {
            eprintln!("attack callback rejected: {error}");
        }
    });
    // SAFETY: Static slot keeps every actual registration outcome, never forgets tickets.
    let installed = unsafe { TASKS.install(engine, [phase], [callback]) }?;
    if !installed.registration?.registered_all() {
        return Err(Error::NativeOperationRejected(
            "partial attack registration retained",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintain_retains_all_seven_opacity_fields_and_never_replays_animation_or_placement() {
        let s = AttackSummonSettings::default();
        let config = s.summons[0];
        let steps = plan(Mode::Maintain, config, &s, None);
        assert_eq!(
            steps
                .iter()
                .filter(|s| matches!(s, Step::Scalar(_, 1.0)))
                .count(),
            7
        );
        assert!(steps.contains(&Step::Team(config.generated_team_type(&s))));
        assert!(steps.contains(&Step::Lock(config.disable_lock_on(&s))));
        assert!(
            !steps
                .iter()
                .any(|s| matches!(s, Step::Place(_) | Step::Animation(_, _) | Step::Hp(_)))
        );
    }
    #[test]
    fn reuse_moves_once_and_preserves_configured_animation_minus_one() {
        let s = AttackSummonSettings::default();
        let mut config = s.summons[2];
        config.animation_id = -1;
        let position = [1., 2., 3., 0.];
        let steps = plan(Mode::Reuse, config, &s, Some(position));
        assert_eq!(
            steps.iter().filter(|s| matches!(s, Step::Place(_))).count(),
            1
        );
        assert!(
            steps.contains(&Step::Place(position))
                && steps.contains(&Step::Animation(-1, true))
                && steps.contains(&Step::Hp(false))
        );
        assert!(
            steps.iter().position(|s| matches!(s, Step::Place(_)))
                < steps.iter().position(|s| matches!(s, Step::Hp(_)))
        );
    }
    #[test]
    fn park_and_soft_release_have_distinct_hp_animation_resource_and_opacity_requests() {
        let s = AttackSummonSettings::default();
        let config = s.summons[1];
        let park = plan(Mode::Park, config, &s, None);
        let release = plan(Mode::Release, config, &s, None);
        assert!(
            park.contains(&Step::Hp(false))
                && park.contains(&Step::ClearAnimation)
                && park.contains(&Step::Present)
                && park.contains(&Step::Updates(true))
        );
        assert!(!park.contains(&Step::Unload));
        assert!(
            release.contains(&Step::Hp(true))
                && release.contains(&Step::Death)
                && release.contains(&Step::Activation(false))
                && release.contains(&Step::Updates(false))
        );
        assert_eq!(release.last(), Some(&Step::Unload));
        for steps in [park, release] {
            assert_eq!(
                steps
                    .iter()
                    .filter(|s| matches!(s, Step::Scalar(_, 0.0)))
                    .count(),
                4
            );
            assert!(steps.contains(&Step::Model(false)));
        }
    }
}
