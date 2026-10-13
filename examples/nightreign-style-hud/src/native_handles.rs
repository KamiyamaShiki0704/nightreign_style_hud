//! Rust-owned opaque values. Never cast a native character to this type.
use shikifsrs_elden_ring::character_handles::NativeCharacterHandle;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HudHandle {
    raw: u64,
}
impl HudHandle {
    pub const fn from_native(raw: u64) -> Self {
        Self { raw }
    }
    pub const fn raw(self) -> u64 {
        self.raw
    }
    #[cfg(test)]
    pub const fn none() -> Self {
        Self { raw: u64::MAX }
    }
    pub fn is_empty(self) -> bool {
        NativeCharacterHandle(self.raw).is_absent()
    }
    pub fn is_bullet(self) -> bool {
        NativeCharacterHandle(self.raw).kind() == 3
    }
    pub fn diagnostic_indices(self) -> Option<(u32, u32)> {
        crate::sdk_host::with_objects(|objects, _| {
            let value = objects.decode_character_handle(NativeCharacterHandle(self.raw))?;
            Ok((value.decoded.set_index, value.decoded.ordinal))
        })
        .ok()
    }
    pub fn diagnostic_set_index(self) -> u32 {
        self.diagnostic_indices().map_or(u32::MAX, |x| x.0)
    }
    pub fn diagnostic_ordinal(self) -> u32 {
        self.diagnostic_indices().map_or(u32::MAX, |x| x.1)
    }
    /// Raw upper-word bytes, most significant first. Consumer display labels remain policy.
    /// Native 508970 uses the high byte as the area-set routing key.
    pub fn location_bytes(self) -> [u8; 4] {
        (self.raw >> 32).to_be_bytes()[4..].try_into().unwrap()
    }
    pub fn passive_seed(self) -> u64 {
        let bytes = self.location_bytes();
        self.raw as u32 as u64 ^ (u64::from(bytes[0]) << 32) ^ (u64::from(bytes[1]) << 40)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_identity_preserves_full_value_and_native_absence_without_layout_casts() {
        let a = HudHandle::from_native(0x3c2a240010020003);
        let b = HudHandle::from_native(0x3c2b240010020003);
        assert_ne!(a, b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 2);
        assert_eq!(a.raw(), 0x3c2a240010020003);
        assert_eq!(a.location_bytes(), [60, 42, 36, 0]);
        assert_eq!(a.passive_seed(), 0x10020003 ^ (60u64 << 32) ^ (42u64 << 40));
        assert!(HudHandle::none().is_empty());
        assert!(HudHandle::from_native(0x12345678ffffffff).is_empty());
        assert!(!a.is_empty());
        assert!(HudHandle::from_native(0x30000001).is_bullet());
        assert!(!a.is_bullet());
    }
}
