//! Actual sparse SDK HUD operations. UI/native allocation ownership remains with the engine.
use shikifsrs_elden_ring::{
    current_frontend_hud::CurrentFrontendHud,
    current_summon_hud::CurrentSummonHud,
    friendly_hud::{FrontendHudObservation, FrontendHudWrite, HudSelection},
    summon_hud::SummonHudWrite,
};
use shikifsrs_runtime::{Error, Result};
use std::{cell::RefCell, collections::VecDeque};

enum Receipt {
    Announcement(Box<SummonHudWrite>),
    Gauges(Box<FrontendHudWrite>),
}
impl Receipt {
    fn failed(&self) -> bool {
        match self {
            Self::Announcement(v) => v.postflight.is_err(),
            Self::Gauges(v) => v.postflight.is_err(),
        }
    }
    fn stores(&self) -> usize {
        match self {
            Self::Announcement(v) => v.stores.len(),
            Self::Gauges(v) => v.stores.len(),
        }
    }
}
struct Session {
    epoch: u64,
    patched: bool,
    messages: CurrentSummonHud,
    frontend: CurrentFrontendHud,
    receipts: VecDeque<Receipt>,
    blocked: bool,
}
impl Session {
    fn retain(&mut self, receipt: Receipt) -> Result<()> {
        let failed = receipt.failed();
        let stores = receipt.stores();
        if self.receipts.len() == 64 {
            self.receipts.pop_front();
        }
        self.receipts.push_back(receipt);
        if failed {
            self.blocked = true;
            eprintln!(
                "Shiki HUD suppression postflight failed after {stores} actual stores; receipt retained"
            );
            return Err(Error::NativeOperationRejected(
                "HUD suppression session blocked",
            ));
        }
        Ok(())
    }
    /// # Safety
    /// Actual callback must retain exclusive live frontend/queue graph through both operations.
    unsafe fn suppress(&mut self, handles: &[u64]) -> Result<()> {
        if self.blocked {
            return Err(Error::NativeOperationRejected(
                "HUD suppression session blocked",
            ));
        }
        let operation = (|| {
            if let Some(source) = self.messages.observe()? {
                // SAFETY: Forwarded caller obligation; one actual store plan, no rollback.
                self.retain(Receipt::Announcement(Box::new(unsafe {
                    self.messages.compare_suppress(source)
                }?)))?;
            }
            if let Some(source) = self.frontend.observe()? {
                // SAFETY: The owned UI policy selects full native handles/color/downscale.
                let receipt = unsafe {
                    self.frontend
                        .compare_suppress(source, selection(handles), true)
                }?;
                self.retain(Receipt::Gauges(Box::new(receipt)))?;
            }
            Ok(())
        })();
        if operation.is_err() {
            self.blocked = true;
        }
        operation
    }
}
fn selection(handles: &[u64]) -> HudSelection<'_> {
    HudSelection {
        identifiers: handles,
        name_color: Some(1),
        include_downscaled: true,
    }
}
#[derive(Default)]
struct Sessions {
    current: Option<Session>,
    old: Vec<Session>,
}
thread_local! {static SESSIONS:RefCell<Sessions>=RefCell::new(Sessions::default());}
fn with_session<R>(run: impl FnOnce(&mut Session) -> Result<R>) -> Result<R> {
    SESSIONS.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| Error::NativeOperationRejected("recursive HUD suppression"))?;
        let epoch = crate::sdk_host::epoch();
        let patched = crate::sdk_host::patches().is_some();
        if slot
            .current
            .as_ref()
            .is_some_and(|s| s.epoch != epoch || s.patched != patched)
        {
            if slot.old.len() == 64 {
                return Err(Error::NativeOperationRejected(
                    "HUD receipt quarantine full",
                ));
            }
            let old = slot.current.take().unwrap();
            slot.old.push(old);
        }
        if slot.current.is_none() {
            let (bytes, base) = crate::sdk_host::binding()?;
            // SAFETY: Host pins the DLL and identifies the exact image; caller retains native graph.
            let messages = unsafe {
                CurrentSummonHud::bind(bytes.clone(), base, epoch, crate::sdk_host::patches())
            }?;
            let frontend = unsafe {
                CurrentFrontendHud::bind(bytes, base, epoch, crate::sdk_host::patches())
            }?;
            slot.current = Some(Session {
                epoch,
                patched,
                messages,
                frontend,
                receipts: VecDeque::new(),
                blocked: false,
            });
        }
        run(slot.current.as_mut().unwrap())
    })
}
/// # Safety
/// Caller supplies the actual exclusive HUD phase and full writable graph lifetime.
pub unsafe fn suppress(handles: &[u64]) -> Result<()> {
    // SAFETY: Explicit obligation passed to the one actual session operation.
    with_session(|session| unsafe { session.suppress(handles) })
}
pub fn observe() -> Result<Option<FrontendHudObservation>> {
    crate::sdk_host::with_objects(|objects, epoch| objects.frontend_hud(epoch))
}
pub fn markers() -> Result<Option<shikifsrs_elden_ring::frontend_markers::FrontendMarkerObservation>>
{
    crate::sdk_host::with_objects(|objects, epoch| objects.frontend_markers(epoch))
}
fn display_marker_position(
    source: &shikifsrs_elden_ring::frontend_markers::FrontendMarkerObservation,
    handle: u64,
) -> Option<[f32; 2]> {
    source
        .displays
        .iter()
        .find(|record| {
            record.identifier == handle
                && record.visible != 0
                && f32::from_bits(record.elapsed_bits) <= 0.25
        })
        .map(|record| record.screen_bits.map(f32::from_bits))
}
fn tag_marker_position(
    source: &shikifsrs_elden_ring::frontend_markers::FrontendMarkerObservation,
    handle: u64,
) -> Option<[f32; 2]> {
    source
        .tags
        .iter()
        .find(|record| {
            record.identifier == handle
                && record.flags[0] != 0
                && record.flags[1] != 0
                && record.flags[2] == 0
        })
        .map(|record| record.screen_pixels.map(|value| value as f32))
}
pub fn enemy_display_position(handle: u64) -> Result<Option<[f32; 2]>> {
    Ok(markers()?.and_then(|source| display_marker_position(&source, handle)))
}
pub fn frontend_tag_position(handle: u64) -> Result<Option<[f32; 2]>> {
    Ok(markers()?.and_then(|source| tag_marker_position(&source, handle)))
}

fn ratio(current: u32, maximum: u32) -> Option<f32> {
    (maximum != 0).then(|| (current as f32 / maximum as f32).clamp(0.0, 1.0))
}
fn copied_buddy_ratio(source: &FrontendHudObservation, handle: u64) -> Option<f32> {
    // Preserve the consumer's spirit -> display -> tag lookup order and full 64-bit ID.
    // Native 771a20 writes spirit current at+8 and uncapped maximum at+14.
    for spirit in &source.spirits {
        if spirit.identifier == handle {
            return ratio(spirit.hp_words[0], spirit.hp_words[3]);
        }
    }
    for display in &source.displays {
        if display.identifier == handle {
            // 778200 publishes78=uncapped,7c=current,80=recoverable,84=uncapped-capped.
            let uncapped = display.hp_words[0];
            let capped = uncapped.wrapping_sub(display.hp_words[3]);
            return ratio(display.hp_words[1], capped.max(uncapped));
        }
    }
    source
        .tags
        .iter()
        .find(|tag| tag.identifier == handle)
        .and_then(|tag| ratio(tag.hp_words[0], tag.hp_words[3]))
}
pub fn buddy_hp_ratio(handle: u64) -> Result<Option<f32>> {
    if handle == u64::MAX {
        return Ok(None);
    }
    Ok(observe()?
        .as_ref()
        .and_then(|source| copied_buddy_ratio(source, handle)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_markers_keep_full_handle_first_match_freshness_and_tag_admission() {
        use shikifsrs_elden_ring::frontend_markers::{
            DisplayMarker, FrontendMarkerObservation, TagMarker,
        };
        let target = 0x1234567800000042;
        let wrong = 0x8765432100000042;
        let mut source = FrontendMarkerObservation {
            module_base: 1,
            epoch: 2,
            frontend: shikifsrs_elden_ring::objects::ObjectObservation {
                address: 3,
                kind: shikifsrs_elden_ring::objects::ObjectKind::Frontend,
            },
            values_address: 4,
            displays: [DisplayMarker {
                address: 5,
                identifier: wrong,
                visible: 255,
                screen_bits: [12.5f32.to_bits(), 37.25f32.to_bits()],
                elapsed_bits: 0.25f32.to_bits(),
            }; 7],
            tags: [TagMarker {
                address: 6,
                identifier: wrong,
                flags: [1, 255, 0],
                screen_pixels: [-32, 45],
            }; 7],
        };
        assert_eq!(display_marker_position(&source, target), None);
        assert_eq!(tag_marker_position(&source, target), None);
        source.displays[6].identifier = target;
        assert_eq!(
            display_marker_position(&source, target),
            Some([12.5, 37.25])
        );
        source.displays[6].elapsed_bits = f32::from_bits(0.25f32.to_bits() + 1).to_bits();
        assert_eq!(display_marker_position(&source, target), None);
        source.displays[6].elapsed_bits = f32::NAN.to_bits();
        assert_eq!(display_marker_position(&source, target), None);
        source.displays[6].elapsed_bits = (-0.5f32).to_bits(); // Preserve original comparison.
        assert_eq!(
            display_marker_position(&source, target),
            Some([12.5, 37.25])
        );
        source.displays[6].visible = 0;
        assert_eq!(display_marker_position(&source, target), None);
        source.tags[6].identifier = target;
        assert_eq!(tag_marker_position(&source, target), Some([-32., 45.]));
        for flags in [[0, 1, 0], [1, 0, 0], [1, 1, 1]] {
            source.tags[6].flags = flags;
            assert_eq!(tag_marker_position(&source, target), None);
        }
        source.tags[6].flags = [1, 1, 0];
        source.tags[0] = source.tags[6];
        source.tags[0].screen_pixels = [21, 34];
        assert_eq!(tag_marker_position(&source, target), Some([21., 34.]));
    }

    #[test]
    fn revenant_selection_keeps_full_handle_and_owned_color_downscale_policy() {
        let known = [0x1234567800000001];
        let s = selection(&known);
        assert!(s.matches(known[0], 0, 0));
        assert!(!s.matches(0x8765432100000001, 0, 0));
        assert!(s.matches(55, 1, 0));
        assert!(s.matches(55, 0, 255));
        assert!(!s.matches(55, 0, 0));
    }
    #[test]
    fn buddy_gauges_keep_native_word_order_full_id_and_original_source_priority() {
        use shikifsrs_elden_ring::friendly_hud::{FriendlyDisplay, FriendlyTag, SpiritDisplay};
        let target = 0x1234567800000001;
        let wrong = 0x8765432100000001;
        let mut source = FrontendHudObservation {
            module_base: 1,
            epoch: 2,
            frontend: shikifsrs_elden_ring::objects::ObjectObservation {
                address: 3,
                kind: shikifsrs_elden_ring::objects::ObjectKind::Frontend,
            },
            values_address: 4,
            spirit_count: 1,
            spirits: [SpiritDisplay {
                address: 5,
                identifier: wrong,
                hp_words: [60, 0, 0, 100],
            }; 5],
            displays: [FriendlyDisplay {
                address: 6,
                visible: 0,
                offscreen: 0,
                name_color: 0,
                downscaled: 0,
                identifier: wrong,
                hp_words: [100, 30, 90, 20],
                damage_delta_bits: 0,
            }; 7],
            tags: [FriendlyTag {
                address: 7,
                flags: [0; 3],
                name_color: 0,
                downscaled: 0,
                identifier: wrong,
                hp_words: [20, 90, 0, 100],
                last_damage: 0,
                last_hp: 0,
            }; 7],
        };
        assert_eq!(copied_buddy_ratio(&source, target), None);
        source.tags[0].identifier = target;
        assert_eq!(copied_buddy_ratio(&source, target), Some(0.2));
        source.displays[0].identifier = target;
        assert_eq!(copied_buddy_ratio(&source, target), Some(0.3));
        source.spirits[0].identifier = target;
        assert_eq!(copied_buddy_ratio(&source, target), Some(0.6));
        source.spirits[0].hp_words[3] = 0;
        assert_eq!(copied_buddy_ratio(&source, target), None);
        source.spirits[0].identifier = wrong;
        source.displays[0].hp_words = [100, 200, 250, 20];
        assert_eq!(copied_buddy_ratio(&source, target), Some(1.0));
        assert_eq!(ratio(10, 0), None);
        source.displays[0].hp_words = [100, 50, 999, (-100i32) as u32];
        assert_eq!(copied_buddy_ratio(&source, target), Some(0.25));
    }
}
