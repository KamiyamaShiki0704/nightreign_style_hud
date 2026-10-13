//! SDK-owned world candidates and actual full-handle native routing receipts.
use crate::sdk_host;
use shikifsrs_elden_ring::{
    character_handles::NativeCharacterHandle,
    character_sets::{CharacterIdentity, WorldCollectionSelection},
    current_character_lookup::{
        CharacterLookupSource, CurrentCharacterLookup, NativeCharacterLookupCall,
    },
    world_update_list::{WorldUpdateEntry, WorldUpdateList},
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};

/// The actual primary-player target snapshot, not a native PlayerIns reference.
pub fn primary_selection()
-> Result<Option<shikifsrs_elden_ring::character_handles::PrimaryPlayerTargetObservation>> {
    sdk_host::with_objects(|objects, epoch| objects.primary_player_target(epoch))
}

pub fn world() -> Result<Option<shikifsrs_elden_ring::objects::ObjectObservation>> {
    sdk_host::with_objects(|objects, _| objects.world())
}

fn checked_entries(list: WorldUpdateList, world: u64, epoch: u64) -> Result<Vec<WorldUpdateEntry>> {
    if list.world.address != world
        || list.epoch != epoch
        || list
            .entries
            .iter()
            .any(|entry| entry.character.world_address != world || entry.character.epoch != epoch)
    {
        return Err(Error::NativeOperationRejected(
            "HUD update-list world or epoch changed",
        ));
    }
    // Preserve native order, duplicates and opaque priority/class. Never sort or classify here.
    Ok(list.entries)
}
pub fn update_entries(world: u64) -> Result<Vec<WorldUpdateEntry>> {
    sdk_host::with_objects(|objects, epoch| {
        let list =
            objects
                .world_update_list(epoch, 65536)?
                .ok_or(Error::NativeOperationRejected(
                    "HUD update-list world absent",
                ))?;
        checked_entries(list, world, epoch)
    })
}
/// Copy actual collection membership without manufacturing references into native storage.
pub fn collection(
    world: u64,
    selection: WorldCollectionSelection,
) -> Result<Vec<CharacterIdentity>> {
    sdk_host::with_objects(|objects, epoch| {
        let snapshot = objects.world_collection(selection, epoch, 4096)?.ok_or(
            Error::NativeOperationRejected("HUD collection world absent"),
        )?;
        if snapshot.world_address != world || snapshot.epoch != epoch {
            return Err(Error::NativeOperationRejected(
                "HUD collection scope changed",
            ));
        }
        snapshot
            .set
            .characters
            .into_iter()
            .map(|member| {
                let target = CharacterIdentity {
                    address: member.character.address,
                    world_address: world,
                    native_identifier: member.native_identifier,
                    epoch,
                };
                if objects.character_identity(target.address, epoch)? != Some(target) {
                    return Err(Error::NativeOperationRejected(
                        "HUD collection identity changed",
                    ));
                }
                Ok(target)
            })
            .collect()
    })
}
/// Copied identity revalidation does not acquire a native borrowing or mutation lease.
pub fn revalidate(target: CharacterIdentity) -> Result<()> {
    sdk_host::with_objects(|objects, epoch| {
        if epoch != target.epoch
            || objects.character_identity(target.address, epoch)? != Some(target)
        {
            return Err(Error::NativeOperationRejected(
                "HUD world candidate superseded",
            ));
        }
        Ok(())
    })
}
struct Attempt {
    source: CharacterLookupSource,
    returned: Option<Result<NativeCharacterLookupCall>>,
}
struct Session {
    epoch: u64,
    patched: bool,
    access: CurrentCharacterLookup,
    attempts: VecDeque<Attempt>,
    blocked: bool,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
}
thread_local! { static SESSIONS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
/// # Safety
/// Retain the full world/area/block/set/character and virtual/native graph in the exact exclusive
/// permitted callback phase. Copied identities do not authorize later native access or acquire a lease.
pub unsafe fn resolve(world: u64, handle: u64) -> Result<Option<CharacterIdentity>> {
    SESSIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD world lookup"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|session| session.epoch != epoch || session.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected("HUD lookup quarantine full"));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Host pins the DLL; the public caller supplies the complete native graph/phase.
            let access = unsafe { CurrentCharacterLookup::bind(bytes, base, epoch, patches) }?;
            slot.current = Some(Session {
                epoch,
                patched,
                access,
                attempts: VecDeque::new(),
                blocked: false,
            });
        }
        let session = slot.current.as_mut().unwrap();
        if session.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD native lookup blocked; actual attempts retained",
            ));
        }
        let source = session
            .access
            .observe(NativeCharacterHandle(handle))?
            .ok_or(Error::NativeOperationRejected(
                "HUD native lookup world absent",
            ))?;
        if source.world.address != world {
            return Err(Error::NativeOperationRejected(
                "HUD native lookup caller world changed",
            ));
        }
        if session.attempts.len() == 64 {
            session.attempts.pop_front();
        }
        session.attempts.push_back(Attempt {
            source,
            returned: None,
        });
        // Publish pending before entry, and remain blocked if an unwind interrupts dispatch.
        session.blocked = true;
        // SAFETY: Forward the caller's full graph/phase; actual return and postflight are retained.
        let returned = unsafe { session.access.lookup(source) };
        let outcome = returned
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|receipt| receipt.postflight.clone());
        let attempt = session.attempts.back_mut().unwrap();
        attempt.returned = Some(returned);
        if outcome.is_ok() {
            session.blocked = false;
        }
        if let Err(error) = &outcome {
            let address = attempt
                .returned
                .as_ref()
                .and_then(|value| value.as_ref().ok())
                .map(|receipt| receipt.returned_address);
            eprintln!(
                "ShikiFSRS HUD lookup retained: source={:?}; RAX={address:?}; {error}",
                attempt.source
            );
        }
        outcome
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::objects::{ObjectKind, ObjectObservation};
    #[test]
    fn candidates_keep_native_order_full_identity_and_raw_priority_and_reject_foreign_scope() {
        let world = 0x600000;
        let epoch = 7;
        let entry = |ordinal, address, identifier, priority_bits, update_class| WorldUpdateEntry {
            ordinal,
            character: CharacterIdentity {
                address,
                world_address: world,
                native_identifier: identifier,
                epoch,
            },
            priority_bits,
            update_class,
        };
        let entries = vec![
            entry(0, 0x620000, 0x3c2a240010020003, 0x7fc01234, -1),
            entry(1, 0x610000, 0x3c2b240010020003, 0x80000000, 7),
        ];
        let list = WorldUpdateList {
            world: ObjectObservation {
                address: world,
                kind: ObjectKind::WorldCharacters,
            },
            epoch,
            entries_address: 0xe00000,
            capacity: 10,
            entries: entries.clone(),
        };
        assert_eq!(
            checked_entries(list.clone(), world, epoch).unwrap(),
            entries
        );
        assert!(checked_entries(list.clone(), world + 8, epoch).is_err());
        assert!(checked_entries(list.clone(), world, epoch + 1).is_err());
        let mut changed = list.clone();
        changed.entries[1].character.world_address += 8;
        assert!(checked_entries(changed, world, epoch).is_err());
        let mut changed = list;
        changed.entries[0].character.epoch += 1;
        assert!(checked_entries(changed, world, epoch).is_err());
    }
}
