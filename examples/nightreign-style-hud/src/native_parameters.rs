//! Independent parameter adapter for the owned HUD. Original role policy stays in state.rs.
use shikifsrs_elden_ring::{
    current_parameters::CurrentParameterAccess,
    parameter_fields::{ParameterField as F, ScalarValue as V},
    parameter_overlays::{FieldKey, ParameterAccess, ParameterOverlays, ReplaceOutcome},
    parameters::ParameterSlot,
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};
const ROW_LIMIT: usize = 65535;
type Edit = (F, V);

pub fn replay_fields(hp: i32, status: [i32; 7]) -> [Edit; 10] {
    [
        (F::EffectHpDelta, V::I32(hp.max(0))),
        (F::EffectPoisonAttack, V::I32(status[0].max(0))),
        (F::EffectDiseaseAttack, V::I32(status[1].max(0))),
        (F::EffectBloodAttack, V::I32(status[2].max(0))),
        (F::EffectCurseAttack, V::I32(status[3].max(0))),
        (F::EffectFreezeAttack, V::I32(status[4].max(0))),
        (F::EffectSleepAttack, V::I32(status[5].max(0))),
        (F::EffectMadnessAttack, V::I32(status[6].max(0))),
        (F::EffectSuperArmorDelta, V::F32(0.0)),
        (F::EffectSaDelta, V::F32(0.0)),
    ]
}
pub fn buddy_fields(npc: i32) -> [Edit; 4] {
    [
        (F::BuddyNpc, V::I32(npc)),
        (F::BuddyThink, V::I32(npc)),
        (F::BuddyRiddenNpc, V::I32(npc)),
        (F::BuddyRiddenThink, V::I32(npc)),
    ]
}
pub fn stone_fields() -> [Edit; 3] {
    [
        (F::BuddyStoneEliminateTarget, V::U32(0)),
        (F::BuddyStoneSummonedFlag, V::U32(0)),
        (F::BuddyStoneActivateRegion, V::U32(0)),
    ]
}
fn direction_field(table: u32) -> Option<F> {
    match table {
        0 => Some(F::NpcAttackSfxDirection),
        1 => Some(F::PlayerAttackSfxDirection),
        _ => None,
    }
}

struct RowOwner {
    slot: u16,
    id: u32,
    fields: ParameterOverlays,
}
#[derive(Debug)]
struct BatchReceipt {
    id: u32,
    requested: Vec<Edit>,
    completed: Vec<(FieldKey, ReplaceOutcome)>,
    failure: Option<String>,
}
#[derive(Default)]
struct Journal {
    rows: Vec<RowOwner>,
    receipts: VecDeque<BatchReceipt>,
    blocked: bool,
}
impl Journal {
    /// # Safety
    /// The backend caller holds exclusive engine access and confirms schema/extent/lifetimes.
    /// Fields are sequential requests. A failed batch is retained and never automatically retried.
    unsafe fn apply<A: ParameterAccess>(
        &mut self,
        access: &A,
        id: u32,
        edits: &[Edit],
    ) -> Result<()> {
        if self.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD parameter journal requires reconciliation",
            ));
        }
        if edits.is_empty() || edits.len() > 64 {
            return Err(Error::InvalidProfile("invalid HUD parameter batch"));
        }
        let slot = edits[0].0.definition().slot;
        for (index, (field, value)) in edits.iter().enumerate() {
            if field.definition().slot != slot
                || field.definition().scalar != value.scalar_type()
                || edits[..index].iter().any(|(previous, _)| previous == field)
            {
                return Err(Error::InvalidProfile(
                    "mixed table, type or duplicate HUD parameter edit",
                ));
            }
            value.bits()?; // Validate the entire batch before the first actual store.
        }
        let row = match self
            .rows
            .iter()
            .position(|row| row.slot == slot && row.id == id)
        {
            Some(index) => index,
            None => {
                if self.rows.len() == ROW_LIMIT {
                    return Err(Error::InvalidProfile("HUD parameter owner bound exceeded"));
                }
                self.rows.push(RowOwner {
                    slot,
                    id,
                    fields: ParameterOverlays::default(),
                });
                self.rows.len() - 1
            }
        };
        if self.receipts.len() == 64 {
            self.receipts.pop_front();
        }
        self.receipts.push_back(BatchReceipt {
            id,
            requested: edits.to_vec(),
            completed: Vec::new(),
            failure: None,
        });
        for (field, value) in edits {
            let key = FieldKey {
                field: *field,
                variant: 0,
                row_id: id,
            };
            // SAFETY: Forwarded exclusive graph contract; SDK retains original row ownership.
            let result = unsafe { self.rows[row].fields.apply(access, key, *value) };
            let receipt = self
                .receipts
                .back_mut()
                .expect("published before any actual store");
            match result {
                Ok(outcome) => {
                    receipt.completed.push((key, outcome));
                    if outcome != ReplaceOutcome::Applied {
                        self.blocked = true;
                        receipt.failure =
                            Some(format!("parameter batch stopped at {key:?}: {outcome:?}"));
                        return Err(Error::NativeOperationRejected(
                            "HUD parameter compare rejected",
                        ));
                    }
                }
                Err(error) => {
                    self.blocked = true;
                    receipt.failure = Some(error.to_string());
                    return Err(error);
                }
            }
        }
        Ok(())
    }
    fn diagnostic(&self) -> String {
        match self.receipts.back() {
            Some(receipt) => format!(
                "row={} requested={} completed={} failure={:?} owners={}",
                receipt.id,
                receipt.requested.len(),
                receipt.completed.len(),
                receipt.failure,
                self.rows.len()
            ),
            None => format!(
                "owners={} no native parameter batch attempted",
                self.rows.len()
            ),
        }
    }
}
struct Session {
    epoch: u64,
    patched: bool,
    access: CurrentParameterAccess,
    journal: Journal,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
}
thread_local! { static SESSIONS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
fn with_session<R>(operation: impl FnOnce(&mut Session) -> Result<R>) -> Result<R> {
    SESSIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD parameters"))?;
        let epoch = crate::sdk_host::epoch();
        let patches = crate::sdk_host::patches();
        let patched = patches.is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|current| current.epoch != epoch || current.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "HUD parameter epoch quarantine full",
                ));
            }
            let previous = slot.current.take().expect("checked current");
            slot.old.push(previous);
        }
        if slot.current.is_none() {
            let (bytes, base) = crate::sdk_host::binding()?;
            // SAFETY: The actual host callback supplies the experimental PARAM phase and complete
            // resource graph. The fixed file gate precedes any loaded process access.
            let access = unsafe {
                CurrentParameterAccess::bind_with_patches(bytes, base, epoch, ROW_LIMIT, patches)?
            };
            slot.current = Some(Session {
                epoch,
                patched,
                access,
                journal: Journal::default(),
            });
        }
        let current = slot.current.as_mut().expect("initialized");
        let result = operation(current);
        if result.is_err() {
            eprintln!("Shiki HUD parameters: {}", current.journal.diagnostic());
        }
        result
    })
}
pub fn write_fields(id: u32, edits: &[Edit]) -> Result<()> {
    with_session(|session| {
        // SAFETY: Forwarded actual host phase/lifetime obligations; explicit retained journal.
        unsafe { session.journal.apply(&session.access, id, edits) }
    })
}
pub fn attack_direction(table: u32, id: u32) -> Option<i8> {
    let field = direction_field(table)?;
    crate::sdk_host::with_objects(|objects, _| {
        Ok(objects
            .experimental_parameter_field(field, 0, id, ROW_LIMIT)?
            .map(|copy| copy.value))
    })
    .ok()
    .flatten()
    .and_then(|value| match value {
        V::I8(direction) => Some(direction),
        _ => None,
    })
}
pub fn stone_rows() -> Result<Vec<u32>> {
    crate::sdk_host::with_objects(|objects, _| {
        let slot = ParameterSlot::new(F::BuddyStoneEliminateTarget.definition().slot)?;
        let rows =
            objects
                .parameter_rows(slot, 0, ROW_LIMIT)?
                .ok_or(Error::NativeOperationRejected(
                    "BuddyStone table unavailable",
                ))?;
        Ok(rows.rows.into_iter().map(|row| row.id).collect())
    })
}
pub fn row_exists(field: F, id: u32) -> Result<bool> {
    crate::sdk_host::with_objects(|objects, _| {
        let slot = ParameterSlot::new(field.definition().slot)?;
        Ok(objects
            .parameter_rows(slot, 0, ROW_LIMIT)?
            .is_some_and(|rows| rows.rows.iter().any(|row| row.id == id)))
    })
}

pub fn read_field(field: F, id: u32) -> Result<Option<V>> {
    crate::sdk_host::with_objects(|objects, _| {
        Ok(objects
            .experimental_parameter_field(field, 0, id, ROW_LIMIT)?
            .map(|v| v.value))
    })
}
pub fn named_row_exists(table: &str, id: u32) -> Result<bool> {
    let (bytes, _) = crate::sdk_host::binding()?;
    let file = shikifsrs_elden_ring::Ww171File::verify(&bytes)?;
    let slot = file
        .parameter_slot(table)?
        .ok_or(Error::InvalidProfile("unknown named PARAM table"))?;
    crate::sdk_host::with_objects(|objects, _| {
        Ok(objects
            .parameter_rows(slot, 0, ROW_LIMIT)?
            .is_some_and(|v| v.rows.iter().any(|v| v.id == id)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::parameter_overlays::{FieldSnapshot, RowIdentity};
    use std::cell::Cell;
    struct Backend {
        values: RefCell<Vec<FieldSnapshot>>,
        writes: Cell<usize>,
        fail_at: Option<usize>,
    }
    // SAFETY: Original exclusively owned synthetic rows, with identity/bit comparison before mutation.
    unsafe impl ParameterAccess for Backend {
        fn observe(&self, key: FieldKey) -> Result<Option<FieldSnapshot>> {
            Ok(self.values.borrow().iter().find(|v| v.key == key).copied())
        }
        unsafe fn compare_replace(
            &self,
            expected: FieldSnapshot,
            bits: u32,
        ) -> Result<ReplaceOutcome> {
            let index = self.writes.get();
            self.writes.set(index + 1);
            if self.fail_at == Some(index) {
                return Err(Error::NativeOperationRejected("synthetic batch failure"));
            }
            let mut rows = self.values.borrow_mut();
            let Some(row) = rows.iter_mut().find(|v| v.key == expected.key) else {
                return Ok(ReplaceOutcome::Unavailable);
            };
            if row.identity != expected.identity {
                return Ok(ReplaceOutcome::Unavailable);
            }
            if row.bits != expected.bits {
                return Ok(ReplaceOutcome::Conflict);
            }
            row.bits = bits;
            Ok(ReplaceOutcome::Applied)
        }
    }
    fn backend(id: u32, edits: &[Edit], fail_at: Option<usize>) -> Backend {
        Backend {
            values: RefCell::new(
                edits
                    .iter()
                    .map(|(field, _)| FieldSnapshot {
                        key: FieldKey {
                            field: *field,
                            variant: 0,
                            row_id: id,
                        },
                        bits: 7,
                        identity: RowIdentity {
                            epoch: 3,
                            resource: 0x1000,
                            header: 0x2000,
                            index: 0x3000,
                            row: 0x4000,
                        },
                    })
                    .collect(),
            ),
            writes: Cell::new(0),
            fail_at,
        }
    }
    #[test]
    fn replay_keeps_all_seven_status_channels_clamps_negative_damage_and_resets_both_posture_deltas()
     {
        let edits = replay_fields(-9, [-3, 2, 3, 4, 5, 6, 7]);
        let backend = backend(8, &edits, None);
        let mut journal = Journal::default();
        // SAFETY: Exclusively owned synthetic selected row fields.
        unsafe {
            journal.apply(&backend, 8, &edits).unwrap();
        }
        assert_eq!(
            backend
                .values
                .borrow()
                .iter()
                .map(|v| v.bits)
                .collect::<Vec<_>>(),
            [0, 0, 2, 3, 4, 5, 6, 7, 0, 0]
        );
        assert_eq!(journal.receipts[0].completed.len(), 10);
        assert_eq!(journal.rows[0].fields.pending(), 9); // Existing value 7 needs no restoration owner.
    }
    #[test]
    fn failed_batch_retains_actual_prefix_and_blocks_replay_without_extra_writes() {
        let edits = replay_fields(100, [1; 7]);
        let backend = backend(8, &edits, Some(3));
        let mut journal = Journal::default();
        // SAFETY: Synthetic failure leaves all successfully replaced fields in explicit ownership.
        unsafe {
            assert!(journal.apply(&backend, 8, &edits).is_err());
            assert!(journal.apply(&backend, 8, &edits).is_err());
        }
        assert_eq!(backend.writes.get(), 4);
        assert_eq!(journal.receipts[0].completed.len(), 3);
        assert_eq!(journal.rows[0].fields.pending(), 3);
        assert!(journal.receipts[0].failure.is_some());
        assert!(backend.values.borrow()[3..].iter().all(|v| v.bits == 7));
    }
    #[test]
    fn buddy_invalid_id_and_stone_unsigned_fields_keep_all_original_policy_outputs() {
        let edits = buddy_fields(-1);
        let backend = backend(9, &edits, None);
        let mut journal = Journal::default();
        // SAFETY: Original synthetic Buddy row, never process memory.
        unsafe {
            journal.apply(&backend, 9, &edits).unwrap();
        }
        assert!(backend.values.borrow().iter().all(|v| v.bits == u32::MAX));
        assert_eq!(
            stone_fields(),
            [
                (F::BuddyStoneEliminateTarget, V::U32(0)),
                (F::BuddyStoneSummonedFlag, V::U32(0)),
                (F::BuddyStoneActivateRegion, V::U32(0))
            ]
        );
        assert_eq!(direction_field(0), Some(F::NpcAttackSfxDirection));
        assert_eq!(direction_field(1), Some(F::PlayerAttackSfxDirection));
        assert_eq!(direction_field(2), None);
    }
    #[test]
    fn entire_batch_type_table_duplicates_and_nonfinite_values_are_rejected_before_mutation() {
        let edits = replay_fields(3, [1; 7]);
        let backend = backend(8, &edits, None);
        for invalid in [
            vec![(F::EffectHpDelta, V::U32(3))],
            vec![(F::EffectHpDelta, V::I32(3)), (F::BuddyNpc, V::I32(3))],
            vec![(F::EffectHpDelta, V::I32(3)), (F::EffectHpDelta, V::I32(4))],
            vec![(F::EffectSuperArmorDelta, V::F32(f32::NAN))],
        ] {
            let mut journal = Journal::default();
            // SAFETY: Malformed original requests must fail before accessing synthetic row storage.
            assert!(unsafe { journal.apply(&backend, 8, &invalid) }.is_err());
            assert_eq!(backend.writes.get(), 0);
            assert!(journal.rows.is_empty());
            assert!(journal.receipts.is_empty());
        }
    }
}
