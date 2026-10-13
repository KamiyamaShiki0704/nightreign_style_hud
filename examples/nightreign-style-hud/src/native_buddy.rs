//! Actual SDK group copies and retained two-word summon requests in original native storage.
use crate::sdk_host;
use shikifsrs_elden_ring::{
    buddy_groups::{
        BuddyControlObservation, BuddyControlWord, BuddyControlWrite, BuddyGroupsObservation,
    },
    current_buddy_control::CurrentBuddyControl,
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};

pub fn groups(world: u64) -> Result<Option<BuddyGroupsObservation>> {
    sdk_host::with_objects(|objects, epoch| {
        let value = objects.buddy_groups(epoch, 4096, 4096)?;
        if value
            .as_ref()
            .is_some_and(|g| g.control.world.address != world)
        {
            return Err(Error::NativeOperationRejected(
                "HUD buddy caller world changed",
            ));
        }
        Ok(value)
    })
}
/// Preserve native group-key/member order and every nonempty full identifier, including duplicates.
pub fn identities(
    snapshot: &BuddyGroupsObservation,
) -> Vec<shikifsrs_elden_ring::character_sets::CharacterIdentity> {
    snapshot
        .groups
        .iter()
        .flat_map(|group| &group.members)
        .filter_map(|member| member.character)
        .filter(|target| {
            !shikifsrs_elden_ring::character_handles::NativeCharacterHandle(
                target.native_identifier,
            )
            .is_absent()
        })
        .collect()
}
pub fn by_param_id(
    snapshot: &BuddyGroupsObservation,
    id: u32,
) -> Option<shikifsrs_elden_ring::buddy_groups::BuddyMemberObservation> {
    snapshot
        .groups
        .iter()
        .flat_map(|group| &group.members)
        .find(|member| {
            member.buddy_param_id == id as i32
                && member.character.is_some_and(|target| {
                    !shikifsrs_elden_ring::character_handles::NativeCharacterHandle(
                        target.native_identifier,
                    )
                    .is_absent()
                })
        })
        .copied()
}
// Keep attempted inputs even after an uncertain native result.
#[allow(dead_code)]
struct Attempt {
    source: BuddyControlObservation,
    field: BuddyControlWord,
    replacement_bits: u32,
    returned: Option<Result<BuddyControlWrite>>,
}
#[derive(Default)]
struct Ledger {
    attempts: VecDeque<Attempt>,
    blocked: bool,
}
impl Ledger {
    fn request_pair(
        &mut self,
        world: u64,
        epoch: u64,
        trigger_effect: i32,
        mut observe: impl FnMut() -> Result<Option<BuddyControlObservation>>,
        mut dispatch: impl FnMut(
            BuddyControlObservation,
            BuddyControlWord,
            u32,
        ) -> Result<BuddyControlWrite>,
    ) -> Result<()> {
        if self.blocked {
            return Err(Error::NativeOperationRejected("HUD buddy request blocked"));
        }
        for (field, bits) in [
            (BuddyControlWord::ItemCooldown, 0),
            (BuddyControlWord::RequestIdentifier, trigger_effect as u32),
        ] {
            // Failure or interruption before the second entry retains the first store.
            self.blocked = true;
            let source =
                observe()?.ok_or(Error::NativeOperationRejected("HUD buddy manager absent"))?;
            if source.world.address != world || source.epoch != epoch {
                return Err(Error::NativeOperationRejected(
                    "HUD buddy request world/epoch changed",
                ));
            }
            self.blocked = false;
            self.run(source, field, bits, || dispatch(source, field, bits))?;
        }
        Ok(())
    }
    fn run(
        &mut self,
        source: BuddyControlObservation,
        field: BuddyControlWord,
        replacement_bits: u32,
        dispatch: impl FnOnce() -> Result<BuddyControlWrite>,
    ) -> Result<()> {
        if self.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD buddy stores blocked; actual records retained",
            ));
        }
        if self.attempts.len() == 64 {
            self.attempts.pop_front();
        }
        self.attempts.push_back(Attempt {
            source,
            field,
            replacement_bits,
            returned: None,
        });
        // Preserve original source/intent before entry. Unwind leaves pending+blocked, never retry.
        self.blocked = true;
        let result = dispatch();
        self.attempts.back_mut().unwrap().returned = Some(result.clone());
        match result {
            Ok(receipt) => {
                if receipt.source != source
                    || receipt.field != field
                    || receipt.replacement_bits != replacement_bits
                {
                    return Err(Error::NativeOperationRejected(
                        "HUD buddy receipt differs from pending intent",
                    ));
                }
                receipt.postflight?;
                if !receipt.stored {
                    return Err(Error::NativeOperationRejected(
                        "HUD buddy native word not stored",
                    ));
                }
                self.blocked = false;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}
struct Session {
    epoch: u64,
    patched: bool,
    access: CurrentBuddyControl,
    ledger: Ledger,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
}
thread_local! { static SESSIONS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
fn with_session<R>(run: impl FnOnce(&mut Session) -> Result<R>) -> Result<R> {
    SESSIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD buddy request"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|s| s.epoch != epoch || s.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected("HUD buddy quarantine full"));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Shared host pins the callback DLL and image. Public caller retains full graph.
            let access = unsafe { CurrentBuddyControl::bind(bytes, base, epoch, patches) }?;
            slot.current = Some(Session {
                epoch,
                patched,
                access,
                ledger: Ledger::default(),
            });
        }
        run(slot.current.as_mut().unwrap())
    })
}
/// # Safety
/// Retain the exact full live world/manager/set graph and original initialized writable words
/// in the exclusive permitted native callback phase, including allocation epoch/ABA obligations.
/// Actual cooldown reset precedes request storage. Partial outcomes remain recorded and blocked.
/// A successful return reports two stored words only, never an actual summon or native ownership.
pub unsafe fn request(world: u64, trigger_effect: i32) -> Result<()> {
    with_session(|session| {
        let Session {
            access,
            ledger,
            epoch,
            ..
        } = session;
        ledger.request_pair(
            world,
            *epoch,
            trigger_effect,
            || access.observe(),
            |source, field, bits| {
                // SAFETY: Forward public caller's exclusive writable full graph contract.
                unsafe { access.compare_word(source, field, bits) }
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::objects::{ObjectKind, ObjectObservation};
    fn source() -> BuddyControlObservation {
        BuddyControlObservation {
            module_base: 0x140000000,
            world: ObjectObservation {
                address: 0x5000,
                kind: ObjectKind::WorldCharacters,
            },
            epoch: 7,
            manager_address: 0x9000,
            set: ObjectObservation {
                address: 0x5000 + 0x10f90,
                kind: ObjectKind::CharacterSet,
            },
            request_identifier: -1,
            item_cooldown_bits: 1.3f32.to_bits(),
        }
    }
    fn receipt(
        source: BuddyControlObservation,
        field: BuddyControlWord,
        bits: u32,
        postflight: Result<BuddyControlObservation>,
    ) -> BuddyControlWrite {
        BuddyControlWrite {
            source,
            field,
            destination: source.manager_address
                + if field == BuddyControlWord::ItemCooldown {
                    0x90
                } else {
                    0x20
                },
            replacement_bits: bits,
            stored: true,
            postflight,
        }
    }
    #[test]
    fn group_selection_keeps_full_id_duplicates_native_order_signed_parameter_and_null_members() {
        use shikifsrs_elden_ring::{
            buddy_groups::{BuddyGroupObservation, BuddyMemberObservation},
            character_sets::CharacterIdentity,
        };
        let a = CharacterIdentity {
            address: 0x1000,
            world_address: source().world.address,
            native_identifier: 0xfedcba9876543210,
            epoch: 7,
        };
        let b = CharacterIdentity {
            native_identifier: a.native_identifier ^ (1 << 48),
            address: 0x2000,
            ..a
        };
        let member = |target: Option<CharacterIdentity>| BuddyMemberObservation {
            node_address: 0x3000,
            next_address: 0x4000,
            previous_address: 0x4000,
            native_character_address: target.map_or(0, |x| x.address),
            character: target,
            buddy_param_id: -2,
            inactive_request_byte: 0xfe,
        };
        let group = |key, members| BuddyGroupObservation {
            node_address: 0x5000,
            left_address: 0x6000,
            parent_address: 0x6000,
            right_address: 0x6000,
            native_key: key,
            list_head: 0x7000,
            members,
        };
        let snapshot = BuddyGroupsObservation {
            control: source(),
            tree_head: 0x6000,
            root_address: 0x5000,
            first_address: 0x5000,
            last_address: 0x5000,
            groups: vec![
                group(
                    i32::MIN,
                    vec![member(None), member(Some(a)), member(Some(a))],
                ),
                group(
                    i32::MAX,
                    vec![
                        member(Some(b)),
                        member(Some(CharacterIdentity {
                            native_identifier: u64::MAX,
                            ..b
                        })),
                    ],
                ),
            ],
        };
        assert_eq!(identities(&snapshot), [a, a, b]);
        assert_eq!(by_param_id(&snapshot, u32::MAX - 1), Some(member(Some(a))));
        assert_eq!(by_param_id(&snapshot, 2), None);
        assert_eq!(
            by_param_id(&snapshot, u32::MAX - 1)
                .unwrap()
                .inactive_request_byte,
            0xfe
        );
    }
    #[test]
    fn request_preserves_successful_reset_then_failed_store_without_retry_or_receipt_loss() {
        let mut ledger = Ledger::default();
        let before = source();
        let mut reset = before;
        reset.item_cooldown_bits = 0;
        ledger
            .run(before, BuddyControlWord::ItemCooldown, 0, || {
                Ok(receipt(
                    before,
                    BuddyControlWord::ItemCooldown,
                    0,
                    Ok(reset),
                ))
            })
            .unwrap();
        let actual = receipt(
            reset,
            BuddyControlWord::RequestIdentifier,
            123456,
            Err(Error::NativeOperationRejected(
                "synthetic postflight failure",
            )),
        );
        assert!(
            ledger
                .run(reset, BuddyControlWord::RequestIdentifier, 123456, || Ok(
                    actual.clone()
                ))
                .is_err()
        );
        assert_eq!(ledger.attempts.len(), 2);
        assert!(ledger.blocked);
        let pending = &ledger.attempts[1];
        assert_eq!(pending.source, reset);
        assert_eq!(pending.field, BuddyControlWord::RequestIdentifier);
        assert_eq!(pending.replacement_bits, 123456);
        assert_eq!(pending.returned, Some(Ok(actual)));
        assert!(
            ledger
                .run(
                    reset,
                    BuddyControlWord::RequestIdentifier,
                    123456,
                    || panic!("must never redispatch")
                )
                .is_err()
        );
        assert_eq!(ledger.attempts.len(), 2);
    }
    #[test]
    fn second_observation_failure_retains_only_actual_reset_and_blocks_whole_pair() {
        for interrupted in [false, true] {
            let mut ledger = Ledger::default();
            let before = source();
            let mut observations = 0;
            let mut entries = 0;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ledger.request_pair(
                    before.world.address,
                    before.epoch,
                    123456,
                    || {
                        observations += 1;
                        if observations == 2 {
                            if interrupted {
                                panic!("synthetic interrupted observation");
                            }
                            return Err(Error::NativeOperationRejected("synthetic graph changed"));
                        }
                        Ok(Some(before))
                    },
                    |actual, field, bits| {
                        entries += 1;
                        assert_eq!(field, BuddyControlWord::ItemCooldown);
                        let mut reset = actual;
                        reset.item_cooldown_bits = bits;
                        Ok(receipt(actual, field, bits, Ok(reset)))
                    },
                )
            }));
            assert!(result.is_err() || result.unwrap().is_err());
            assert_eq!(entries, 1);
            assert_eq!(observations, 2);
            assert!(ledger.blocked);
            assert_eq!(ledger.attempts.len(), 1);
            assert!(
                ledger.attempts[0]
                    .returned
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .stored
            );
            assert!(
                ledger
                    .request_pair(
                        before.world.address,
                        before.epoch,
                        123456,
                        || panic!("must not observe again"),
                        |_, _, _| panic!("must not retry reset")
                    )
                    .is_err()
            );
        }
    }
    #[test]
    fn interrupted_dispatch_keeps_original_pending_intent_and_blocks_future_entries() {
        let mut ledger = Ledger::default();
        let source = source();
        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ledger.run(
                source,
                BuddyControlWord::RequestIdentifier,
                u32::MAX,
                || panic!("synthetic interrupted callback"),
            )
        }));
        assert!(interrupted.is_err());
        assert!(ledger.blocked);
        assert_eq!(ledger.attempts[0].returned, None);
        assert_eq!(ledger.attempts[0].source, source);
        assert_eq!(ledger.attempts[0].replacement_bits, u32::MAX);
        assert!(
            ledger
                .run(source, BuddyControlWord::ItemCooldown, 0, || panic!(
                    "must never retry"
                ))
                .is_err()
        );
    }
}
