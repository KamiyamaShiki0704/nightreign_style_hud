//! Owned ordinary creation tickets. No global result address is claimed by NPC/model equality.
use crate::{
    hud_creation_queue::{Attempt, Completed, NativeDriver, Queue, Request},
    sdk_host,
};
use shikifsrs_elden_ring::{
    character_sets::CharacterIdentity, creation_input_preparation::OrdinaryCreationInput,
    current_debug_creator::CurrentDebugCreator,
};
use shikifsrs_runtime::{Error, Result};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

const LIMIT: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestId {
    epoch: u64,
    serial: usize,
}
struct Session {
    epoch: u64,
    patched: bool,
    driver: NativeDriver,
    queue: Queue<Attempt>,
    completed: Vec<Completed<Attempt>>,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
    serial: usize,
}
thread_local! { static SESSIONS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
fn with_session<R>(
    world: u64,
    action: impl FnOnce(&mut Session, &mut usize) -> Result<R>,
) -> Result<R> {
    SESSIONS.with(|slot| {
        let mut slots = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD creation"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if slots
            .current
            .as_ref()
            .is_some_and(|s| s.epoch != epoch || s.patched != patched)
        {
            if slots.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "HUD creation quarantine full",
                ));
            }
            let old = slots.current.take().unwrap();
            slots.old.push(old); // Keep every ticket/result and failed prefix; no native Drop cleanup.
        }
        if slots.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Host retains the loaded image; invocation supplies the experimental full graph/phase.
            let creator = unsafe { CurrentDebugCreator::bind(bytes, base, epoch, patches) }?;
            slots.current = Some(Session {
                epoch,
                patched,
                driver: NativeDriver {
                    creator,
                    set_limit: LIMIT,
                    pending: None,
                },
                queue: Queue::new(LIMIT)?,
                completed: Vec::new(),
            });
        }
        let Sessions {
            current, serial, ..
        } = &mut *slots;
        let session = current.as_mut().unwrap();
        let actual = session
            .driver
            .creator
            .observe()?
            .ok_or(Error::NativeOperationRejected("HUD creator absent"))?;
        if actual.world.address != world || actual.epoch != epoch {
            return Err(Error::NativeOperationRejected(
                "HUD creator world superseded",
            ));
        }
        action(session, serial)
    })
}
pub fn busy(world: u64) -> Result<bool> {
    with_session(world, |session, _| {
        if let Some(error) = session.queue.blocked() {
            return Err(error.clone());
        }
        let source = session
            .driver
            .creator
            .observe()?
            .ok_or(Error::NativeOperationRejected("HUD creator absent"))?;
        Ok(session.queue.pending_count() != 0
            || source.last_created_address != 0
            || source.schedule.current_step != 0
            || source.schedule.next_step != 0
            || source.schedule.advance_byte != 0)
    })
}
/// # Safety
/// Retain the full ordinary creator/world/PARAM/resource/allocator graph and exact exclusive
/// callback phase through preparation, native dispatch and postflight. The receipt is experimental.
pub unsafe fn submit(world: u64, input: OrdinaryCreationInput, now: Instant) -> Result<RequestId> {
    with_session(world, |session, serial| {
        if let Some(error) = session.queue.blocked() {
            return Err(error.clone());
        }
        if session.completed.len() + session.queue.pending_count() >= LIMIT {
            return Err(Error::NativeOperationRejected(
                "HUD retained creation ticket limit",
            ));
        }
        let next = serial.checked_add(1).ok_or(Error::AddressOverflow)?;
        session.queue.enqueue(Request {
            config_index: next,
            requested_at: now,
            expires_at: None,
            input,
        })?;
        *serial = next;
        // Only enqueue here: return the owned serial before any native entry can fail.
        Ok(RequestId {
            epoch: session.epoch,
            serial: next,
        })
    })
}
/// # Safety
/// Supply the same complete exclusive native graph/phase as submit, including retained tickets.
pub unsafe fn poll(world: u64, now: Instant) -> Result<()> {
    with_session(world, |session, _| {
        if session.completed.len() >= LIMIT {
            return Err(Error::NativeOperationRejected(
                "HUD retained creation ticket limit",
            ));
        }
        // Shared driver retains preparation/dispatch/clear receipts. Queue latches before begin.
        if let Some(completed) =
            session
                .queue
                .poll(&mut session.driver, now, Duration::from_secs(2))?
        {
            session.completed.push(completed);
        }
        Ok(())
    })
}
pub fn result(world: u64, request: RequestId) -> Result<Option<CharacterIdentity>> {
    with_session(world, |session, _| {
        if request.epoch != session.epoch {
            return Err(Error::NativeOperationRejected(
                "HUD creation request epoch changed",
            ));
        }
        let Some(completed) = session
            .completed
            .iter()
            .find(|c| c.pending.request.config_index == request.serial)
        else {
            return Ok(None);
        };
        let registration = completed.registration;
        sdk_host::with_objects(|objects, _| {
            if objects.character_registration(
                registration.character,
                registration.set_address,
                LIMIT,
            )? != Some(registration)
            {
                return Err(Error::NativeOperationRejected(
                    "HUD created registration superseded",
                ));
            }
            Ok(Some(registration.character))
        })
    })
}
pub fn owned_by_handle(world: u64, handle: u64) -> Result<Option<CharacterIdentity>> {
    with_session(world, |session, _| {
        let Some(completed) = session.completed.iter().find(|c| {
            c.registration.character.world_address == world
                && c.registration.character.native_identifier == handle
        }) else {
            return Ok(None);
        };
        let registration = completed.registration;
        sdk_host::with_objects(|objects, _| {
            if objects.character_registration(
                registration.character,
                registration.set_address,
                LIMIT,
            )? != Some(registration)
            {
                return Err(Error::NativeOperationRejected(
                    "HUD owned creation registration superseded",
                ));
            }
            Ok(Some(registration.character))
        })
    })
}
