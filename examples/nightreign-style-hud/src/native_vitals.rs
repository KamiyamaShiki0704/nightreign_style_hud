//! Conditional SDK stores for the HUD's existing recovery and posture-damage policy.
use crate::sdk_host;
use shikifsrs_elden_ring::{
    character_posture::{CurrentCharacterPosture, NativeSuperArmorMaximumCall},
    character_scalars::{
        CharacterReplaceOutcome, CharacterScalar, CharacterScalarAccess, CharacterScalarKey,
        CharacterScalarSnapshot,
    },
    character_sets::CharacterIdentity,
    character_vitals::{
        CharacterVital, CharacterVitalWrite, CharacterVitalsObservation, CurrentMaximum,
    },
    current_character_scalars::CurrentCharacterScalarAccess,
    current_character_vitals::CurrentCharacterVitals,
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};

#[derive(Debug, Clone, Copy)]
pub enum VitalChange {
    Delta(i32),
    Value(i32),
    Fraction(f32),
    RecoverDead,
}
fn next_vital(source: CurrentMaximum, change: VitalChange) -> Result<i32> {
    let maximum = source.maximum.max(1);
    Ok(match change {
        VitalChange::Delta(delta) => source.current.saturating_add(delta).clamp(0, maximum),
        VitalChange::Value(value) => value,
        VitalChange::RecoverDead => {
            if source.current <= 0 {
                maximum
            } else {
                source.current
            }
        }
        VitalChange::Fraction(fraction) => {
            if !fraction.is_finite() {
                return Err(Error::InvalidProfile("nonfinite HUD health ratio"));
            }
            ((maximum as f32) * fraction.clamp(0., 1.))
                .round()
                .clamp(0., maximum as f32) as i32
        }
    })
}

enum Receipt {
    Vital(CharacterVitalWrite),
    Scalar {
        source: CharacterScalarSnapshot,
        bits: u32,
        outcome: CharacterReplaceOutcome,
        postflight: Result<CharacterScalarSnapshot>,
    },
    Maximum(NativeSuperArmorMaximumCall),
}
impl Receipt {
    fn completion(&self) -> Result<()> {
        match self {
            Self::Vital(value) => value.postflight.as_ref().map(|_| ()).map_err(Clone::clone),
            Self::Maximum(value) => value
                .postflight
                .as_ref()
                .map(|_| ())
                .map_err(Clone::clone)
                .and_then(|_| value.maximum.as_ref().map(|_| ()).map_err(Clone::clone)),
            Self::Scalar {
                source,
                bits,
                outcome,
                postflight,
            } => {
                if *outcome != CharacterReplaceOutcome::Applied {
                    return Err(Error::NativeOperationRejected(
                        "HUD posture conditional store rejected",
                    ));
                }
                let actual = postflight.as_ref().map_err(Clone::clone)?;
                if actual.key != source.key
                    || actual.storage != source.storage
                    || actual.bits != *bits
                {
                    return Err(Error::NativeOperationRejected(
                        "HUD posture store changed after write",
                    ));
                }
                Ok(())
            }
        }
    }
}
struct Session {
    epoch: u64,
    patched: bool,
    vitals: CurrentCharacterVitals,
    scalars: CurrentCharacterScalarAccess,
    posture: CurrentCharacterPosture,
    receipts: VecDeque<Receipt>,
    blocked: bool,
}
impl Session {
    fn retain(&mut self, receipt: Receipt) -> Result<()> {
        let completion = receipt.completion();
        if self.receipts.len() == 64 {
            self.receipts.pop_front();
        }
        self.receipts.push_back(receipt);
        if completion.is_err() {
            self.blocked = true;
        }
        completion
    }
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
            .map_err(|_| Error::NativeOperationRejected("recursive HUD vital operation"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|v| v.epoch != epoch || v.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected("HUD vital quarantine full"));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Host pins this module; actual callback must retain live native graphs and exclusion.
            let vitals = unsafe {
                CurrentCharacterVitals::bind(bytes.clone(), base, epoch, patches.clone())
            }?;
            // SAFETY: The same explicit caller contract covers selected posture destinations.
            let scalars = unsafe {
                CurrentCharacterScalarAccess::bind_with_patches(
                    bytes.clone(),
                    base,
                    epoch,
                    patches.clone(),
                )
            }?;
            // SAFETY: Native maximum query has additional game-data and virtual-call obligations.
            let posture = unsafe { CurrentCharacterPosture::bind(bytes, base, epoch, patches) }?;
            slot.current = Some(Session {
                epoch,
                patched,
                vitals,
                scalars,
                posture,
                receipts: VecDeque::new(),
                blocked: false,
            });
        }
        let session = slot.current.as_mut().unwrap();
        if session.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD vital operation blocked; actual receipt retained",
            ));
        }
        let result = run(session);
        if result.is_err() {
            session.blocked = true;
        }
        result
    })
}
pub fn identity(address: u64) -> Result<CharacterIdentity> {
    sdk_host::with_objects(|objects, epoch| {
        objects
            .character_identity(address, epoch)?
            .ok_or(Error::NativeOperationRejected(
                "HUD character identity absent",
            ))
    })
}
pub fn observe(address: u64) -> Result<CharacterVitalsObservation> {
    let target = identity(address)?;
    sdk_host::with_objects(|objects, _| {
        objects
            .character_vitals(target)?
            .ok_or(Error::NativeOperationRejected("HUD vital storage absent"))
    })
}
/// # Safety
/// The caller retains exclusive writable vital storage and full native character identity lifetime.
/// A failed postflight retains the actual write and blocks subsequent stores; no retry or Drop write.
pub unsafe fn change(address: u64, field: CharacterVital, change: VitalChange) -> Result<i32> {
    let target = identity(address)?;
    with_session(|session| {
        let source = session
            .vitals
            .observe(target)?
            .ok_or(Error::NativeOperationRejected("HUD vital storage absent"))?;
        let before = field.pair(source).current;
        let after = next_vital(field.pair(source), change)?;
        // SAFETY: The caller's exact vital-store contract is forwarded to the SDK.
        let receipt = unsafe { session.vitals.compare_current(source, field, after) }?;
        session.retain(Receipt::Vital(receipt))?;
        Ok(after.saturating_sub(before))
    })
}
/// # Safety
/// Caller supplies stable exclusive ownership of both selected posture fields; stores are sequential.
pub unsafe fn damage_posture(address: u64, damage: f32) -> Result<()> {
    if !damage.is_finite() {
        return Err(Error::InvalidProfile("nonfinite HUD posture damage"));
    }
    if damage <= 0. {
        return Ok(());
    }
    let target = identity(address)?;
    with_session(|session| {
        for field in [
            CharacterScalar::ToughnessCurrent,
            CharacterScalar::SuperArmorCurrent,
        ] {
            let key = CharacterScalarKey {
                character: target,
                field,
            };
            let source = session
                .scalars
                .observe(key)?
                .ok_or(Error::NativeOperationRejected("HUD posture storage absent"))?;
            let before = f32::from_bits(source.bits);
            if !before.is_finite() {
                return Err(Error::InvalidProfile("nonfinite HUD posture baseline"));
            }
            let bits = (before - damage).max(0.).to_bits();
            // SAFETY: Caller guarantees exact live destination and exclusive selected field access.
            let outcome = unsafe { session.scalars.compare_replace(source, bits) }?;
            let postflight = session.scalars.observe(key).and_then(|v| {
                v.ok_or(Error::NativeOperationRejected(
                    "HUD posture absent after store",
                ))
            });
            session.retain(Receipt::Scalar {
                source,
                bits,
                outcome,
                postflight,
            })?;
        }
        Ok(())
    })
}
/// # Safety
/// This calls the native maximum virtual function, including Player game-data refresh. It is not read-only.
pub unsafe fn super_armor_values(address: u64) -> Result<(f32, f32)> {
    let target = identity(address)?;
    with_session(|session| {
        let source = session
            .posture
            .observe(
                target,
                shikifsrs_elden_ring::character_posture::PostureKind::SuperArmor,
            )?
            .ok_or(Error::NativeOperationRejected("HUD super armor absent"))?;
        // SAFETY: Caller additionally supplies the full virtual-call and game-data graph contract.
        let receipt = unsafe { session.posture.query_super_armor_maximum(source) }?;
        let values = receipt
            .maximum
            .clone()
            .map(|maximum| (f32::from_bits(source.current_bits), maximum));
        session.retain(Receipt::Maximum(receipt))?;
        values
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_keeps_owned_signed_delta_and_fraction_policies() {
        let pair = CurrentMaximum {
            current: 50,
            maximum: 100,
        };
        assert_eq!(next_vital(pair, VitalChange::Delta(i32::MAX)).unwrap(), 100);
        assert_eq!(next_vital(pair, VitalChange::Delta(i32::MIN)).unwrap(), 0);
        assert_eq!(next_vital(pair, VitalChange::Fraction(0.335)).unwrap(), 34);
        assert!(next_vital(pair, VitalChange::Fraction(f32::NAN)).is_err());
        assert_eq!(next_vital(pair, VitalChange::RecoverDead).unwrap(), 50);
        assert_eq!(
            next_vital(
                CurrentMaximum {
                    current: -1,
                    maximum: 0
                },
                VitalChange::RecoverDead
            )
            .unwrap(),
            1
        );
        assert_eq!(next_vital(pair, VitalChange::Value(0)).unwrap(), 0);
    }
}
