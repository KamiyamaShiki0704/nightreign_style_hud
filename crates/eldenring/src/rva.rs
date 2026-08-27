use pelite::pe64::PeView;
use std::sync::LazyLock;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::PCSTR;

mod bundle;
mod rva_jp;
mod rva_ww;

pub use bundle::RvaBundle;

use fromsoftware_shared::game_version::{GameVersion, LANG_ID_EN, LANG_ID_JP};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ERGameVersion {
    Ww262,
    Jp2621,
    Ww2700,
    Jp2701,
}

impl GameVersion for ERGameVersion {
    const NAME: &'static str = "elden ring";

    fn from_lang_version(lang_id: u16, version: &str) -> Option<Self> {
        match (lang_id, version) {
            (LANG_ID_EN, "2.6.2.0") => Some(Self::Ww262),
            (LANG_ID_JP, "2.6.2.1") => Some(Self::Jp2621),
            (LANG_ID_EN, "2.7.0.0") => Some(Self::Ww2700),
            (LANG_ID_JP, "2.7.0.1") => Some(Self::Jp2701),
            _ => None,
        }
    }
}

impl ERGameVersion {
    const fn rvas(self) -> RvaBundle {
        match self {
            Self::Ww262 => rva_ww::RVAS_262,
            Self::Jp2621 => rva_jp::RVAS_2621,
            Self::Ww2700 => rva_ww::RVAS_2700,
            Self::Jp2701 => rva_jp::RVAS_2701,
        }
    }
}

/// Returns the RVA bundle for the current executable region and version.
///
/// This will panic if the current executable isn't supported by this package.
pub fn get() -> &'static RvaBundle {
    static RVAS: LazyLock<RvaBundle> = LazyLock::new(|| {
        let module = unsafe {
            PeView::module(GetModuleHandleA(PCSTR(std::ptr::null())).unwrap().0 as *const u8)
        };
        ERGameVersion::detect(&module)
            .unwrap_or_else(|e| panic!("{e}"))
            .rvas()
    });

    &RVAS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_retained_and_current_executable_versions() {
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_EN, "2.6.2.0"),
            Some(ERGameVersion::Ww262)
        );
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_JP, "2.6.2.1"),
            Some(ERGameVersion::Jp2621)
        );
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_EN, "2.7.0.0"),
            Some(ERGameVersion::Ww2700)
        );
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_JP, "2.7.0.1"),
            Some(ERGameVersion::Jp2701)
        );
    }

    #[test]
    fn rejects_unmapped_executable_versions() {
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_EN, "2.7.0.1"),
            None
        );
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_JP, "2.7.0.0"),
            None
        );
        assert_eq!(
            ERGameVersion::from_lang_version(LANG_ID_EN, "2.8.0.0"),
            None
        );
    }

    #[test]
    fn selects_version_specific_bullet_spawn_rvas() {
        assert_eq!(
            ERGameVersion::Ww262.rvas().cs_bullet_manager_spawn_bullet,
            0x3a2ca0
        );
        assert_eq!(
            ERGameVersion::Jp2621.rvas().cs_bullet_manager_spawn_bullet,
            0x3a2ca0
        );
        assert_eq!(
            ERGameVersion::Ww2700.rvas().cs_bullet_manager_spawn_bullet,
            0x3a2cb0
        );
        assert_eq!(
            ERGameVersion::Jp2701.rvas().cs_bullet_manager_spawn_bullet,
            0x3a2cb0
        );
    }
}
