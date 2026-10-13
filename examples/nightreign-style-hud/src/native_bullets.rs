//! Copied bullets and retained actual mutation attempts. No native references escape.
use crate::sdk_host;
use shikifsrs_elden_ring::{
    bullet_spawn::{BulletSpawnCall, BulletSpawnObservation, BulletSpawnTargetWrite},
    bullets::{
        BulletListObservation, BulletObservation, BulletVelocityWrite, MAX_PRIMARY_BULLET_MEMBERS,
    },
    current_bullets::CurrentBullets,
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key([u64; 5]);
pub fn key(bullet: &BulletObservation) -> Key {
    let id = bullet.identity;
    Key([
        id.module_base,
        id.manager_address,
        id.address,
        id.native_identifier,
        id.epoch,
    ])
}
pub fn observe() -> Result<BulletListObservation> {
    sdk_host::with_objects(|objects, epoch| {
        objects
            .bullets(epoch, MAX_PRIMARY_BULLET_MEMBERS)?
            .ok_or(Error::NativeOperationRejected("HUD bullet manager absent"))
    })
}
pub fn capture(address: u64) -> Result<BulletSpawnObservation> {
    sdk_host::with_objects(|objects, epoch| objects.bullet_spawn_input(address, epoch))
}
#[derive(Debug)]
enum Source {
    Target(BulletSpawnObservation, u64),
    Velocity(BulletObservation, [f32; 3]),
    Spawn(u64, BulletSpawnObservation),
}
#[derive(Debug)]
enum Receipt {
    Target(Box<BulletSpawnTargetWrite>),
    Velocity(Box<BulletVelocityWrite>),
    Spawn(BulletSpawnCall),
}
impl Receipt {
    fn result(&self) -> Result<()> {
        match self {
            Self::Target(value) => value.postflight.as_ref().map(|_| ()).map_err(Clone::clone),
            Self::Velocity(value) => value.postflight.as_ref().map(|_| ()).map_err(Clone::clone),
            Self::Spawn(value) => {
                value.postflight.as_ref().map_err(Clone::clone)?;
                if !value.returned_output_pointer {
                    return Err(Error::NativeOperationRejected(
                        "HUD bullet spawn returned foreign output pointer",
                    ));
                }
                Ok(())
            }
        }
    }
}
struct Attempt {
    source: Source,
    returned: Option<Result<Receipt>>,
}
#[derive(Default)]
struct Ledger {
    attempts: VecDeque<Attempt>,
    blocked: bool,
}
impl Ledger {
    fn run(
        &mut self,
        source: Source,
        dispatch: impl FnOnce(&Source) -> Result<Receipt>,
    ) -> Result<()> {
        if self.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD bullet operations blocked; actual attempts retained",
            ));
        }
        if self.attempts.len() == 64 {
            self.attempts.pop_front();
        }
        self.attempts.push_back(Attempt {
            source,
            returned: None,
        });
        self.blocked = true;
        let attempt = self.attempts.back_mut().unwrap();
        let returned = dispatch(&attempt.source);
        let result = returned
            .as_ref()
            .map_err(Clone::clone)
            .and_then(Receipt::result);
        attempt.returned = Some(returned);
        if result.is_ok() {
            self.blocked = false;
        }
        result
    }
}
struct Session {
    epoch: u64,
    patched: bool,
    access: CurrentBullets,
    ledger: Ledger,
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
}
thread_local! { static SESSIONS: RefCell<Sessions> = RefCell::new(Sessions::default()); }
fn mutate(source: Source) -> Result<()> {
    SESSIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD bullet operation"))?;
        let epoch = sdk_host::epoch();
        let patches = sdk_host::patches();
        let patched = patches.is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|current| current.epoch != epoch || current.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "HUD bullet generation quarantine full",
                ));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = sdk_host::binding()?;
            // SAFETY: Public unsafe operations require the original full retained graph/phase;
            // the host pins its DLL, code manifest and caller-owned lifetime generation.
            let access = unsafe { CurrentBullets::bind(bytes, base, epoch, patches) }?;
            slot.current = Some(Session {
                epoch,
                patched,
                access,
                ledger: Ledger::default(),
            });
        }
        let session = slot.current.as_mut().unwrap();
        match &source {
            Source::Target(expected, _) | Source::Spawn(_, expected) if expected.epoch != epoch => {
                return Err(Error::NativeOperationRejected("revoked HUD spawn input"));
            }
            Source::Velocity(expected, _) if expected.identity.epoch != epoch => {
                return Err(Error::NativeOperationRejected(
                    "revoked HUD bullet identity",
                ));
            }
            _ => {}
        }
        let access = &session.access;
        session.ledger.run(source, |source| {
            // SAFETY: Actual caller supplies exclusive writable storage and the full allocator,
            // PARAM, character, targeting and helper graph. There is no retry or native Drop.
            unsafe {
                match source {
                    Source::Target(expected, target) => access
                        .compare_spawn_target(expected.clone(), *target)
                        .map(|receipt| Receipt::Target(Box::new(receipt))),
                    Source::Velocity(expected, xyz) => access
                        .compare_velocity(*expected, *xyz, MAX_PRIMARY_BULLET_MEMBERS)
                        .map(|receipt| Receipt::Velocity(Box::new(receipt))),
                    Source::Spawn(manager, source) => access
                        .spawn(*manager, &source.input, MAX_PRIMARY_BULLET_MEMBERS)
                        .map(Receipt::Spawn),
                }
            }
        })
    })
}
/// # Safety
/// Retain exact initialized writable input, full handle graph and exclusive spawn callback phase.
pub unsafe fn retarget(source: BulletSpawnObservation, target: u64) -> Result<()> {
    mutate(Source::Target(source, target))
}
/// # Safety
/// Retain source's full bullet identity, initialized writable XYZ extent and exclusive engine phase.
pub unsafe fn velocity(source: BulletObservation, xyz: [f32; 3]) -> Result<()> {
    mutate(Source::Velocity(source, xyz))
}
/// # Safety
/// This captured input comes from the current callback; retain all its opaque references and the
/// complete native spawn graph. Prevent recursive clone dispatch. Copies acquire no native lease.
pub unsafe fn spawn(manager: u64, input: BulletSpawnObservation) -> Result<()> {
    mutate(Source::Spawn(manager, input))
}
/// Preserve the consumer's original speed fallback, fade and normalized blend; no storage writes.
pub fn guidance_velocity(
    bullet: &BulletObservation,
    target: [f32; 3],
    mix: f32,
) -> Option<[f32; 3]> {
    let mix = mix.clamp(0.0, 1.0);
    if mix <= 0.001 {
        return None;
    }
    let position = bullet.position_bits.map(f32::from_bits);
    let delta = std::array::from_fn::<_, 3, _>(|i| target[i] - position[i]);
    let len = delta.iter().map(|v| v * v).sum::<f32>().sqrt();
    if !len.is_finite() || len < 0.001 {
        return None;
    }
    let velocity = bullet.velocity_bits.map(f32::from_bits);
    let speed = velocity[..3].iter().map(|v| v * v).sum::<f32>().sqrt();
    let speed = if speed.is_finite() && speed > 0.1 {
        speed
    } else {
        24.0
    };
    let original = std::array::from_fn::<_, 3, _>(|i| {
        if speed > 0.001 {
            velocity[i] / speed
        } else {
            delta[i] / len
        }
    });
    let mixed =
        std::array::from_fn::<_, 3, _>(|i| original[i] * (1.0 - mix) + delta[i] / len * mix);
    let length = mixed.iter().map(|v| v * v).sum::<f32>().sqrt();
    if !length.is_finite() || length < 0.001 {
        return None;
    }
    Some(mixed.map(|v| v / length * speed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::bullets::{BulletIdentity, BulletParameterState};
    fn bullet() -> BulletObservation {
        let state = BulletParameterState {
            address: 0x4000,
            row_address: 0x8000,
            parameter_id: 123,
        };
        BulletObservation {
            identity: BulletIdentity {
                module_base: 0x140000000,
                manager_address: 0x2000,
                address: 0x3000,
                native_identifier: 0x1000030000001,
                epoch: 1,
            },
            next_address: 0,
            owner_identifier: 1,
            targeting_owner_identifier: 1,
            position_bits: [0; 4],
            velocity_bits: [4f32.to_bits(), 0, 0, 0x7fc00017],
            elapsed_step_bits: 0,
            wait: state,
            fly: state,
            explosion: state,
        }
    }
    #[test]
    fn full_identity_key_revokes_reused_address_and_preserves_high_handle() {
        let a = bullet();
        for index in 0..5 {
            let mut b = a;
            match index {
                0 => b.identity.module_base += 16,
                1 => b.identity.manager_address += 16,
                2 => b.identity.address += 16,
                3 => b.identity.native_identifier ^= 1 << 48,
                _ => b.identity.epoch += 1,
            }
            assert_ne!(key(&a), key(&b));
        }
    }
    #[test]
    fn direction_keeps_speed_fallback_and_rejects_invalid_geometry() {
        let mut b = bullet();
        assert_eq!(guidance_velocity(&b, [0., 0., 10.], 1.), Some([0., 0., 4.]));
        assert!(guidance_velocity(&b, [0., 0., 10.], 0.).is_none());
        assert!(guidance_velocity(&b, [f32::NAN, 0., 10.], 0.4).is_none());
        b.velocity_bits[..3].fill(0);
        assert_eq!(
            guidance_velocity(&b, [0., 0., 10.], 0.4),
            Some([0., 0., 24.])
        );
        assert_eq!(b.velocity_bits[3], 0x7fc00017);
    }
    #[test]
    fn returned_store_failure_keeps_actual_receipt_and_prevents_second_dispatch() {
        let b = bullet();
        let mut ledger = Ledger::default();
        let error = Error::NativeOperationRejected("owned postflight failure");
        assert!(
            ledger
                .run(Source::Velocity(b, [1., 2., 3.]), |_| Ok(
                    Receipt::Velocity(Box::new(BulletVelocityWrite {
                        source: b,
                        requested_bits: [1f32.to_bits(), 2f32.to_bits(), 3f32.to_bits()],
                        stored: true,
                        postflight: Err(error.clone())
                    }))
                ))
                .is_err()
        );
        let Some(Ok(Receipt::Velocity(receipt))) = &ledger.attempts[0].returned else {
            panic!("actual receipt missing")
        };
        assert!(receipt.stored);
        assert_eq!(receipt.postflight, Err(error));
        assert!(
            ledger
                .run(Source::Velocity(b, [3., 2., 1.]), |_| panic!(
                    "must not redispatch"
                ))
                .is_err()
        );
        assert_eq!(ledger.attempts.len(), 1);
    }
    #[test]
    fn interrupted_dispatch_keeps_pending_attempt_and_closed_session() {
        let b = bullet();
        let mut ledger = Ledger::default();
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = ledger.run(Source::Velocity(b, [1., 2., 3.]), |_| {
                panic!("owned callback unwind")
            });
        }));
        assert!(unwind.is_err());
        assert!(ledger.blocked);
        assert!(ledger.attempts[0].returned.is_none());
        assert!(
            ledger
                .run(Source::Velocity(b, [0.; 3]), |_| panic!(
                    "must not redispatch"
                ))
                .is_err()
        );
    }
}
