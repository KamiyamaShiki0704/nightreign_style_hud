//! Read-only loading request shared by the HUD and native retirement session.
//! WW 2.7.1 evidence: docs/hud-map-refresh/design.md.
use eldenring::cs::CSMenuManImp;
use fromsoftware_shared::{FromStatic, program::Program};
use pelite::pe64::{Pe, PeObject};
use std::sync::LazyLock;

const REQUEST_OFFSET: usize = 0x10;
const FINGERPRINTS: &[(usize, &[u8])] = &[
    // LoadingScreenData::request: set the request word to 1.
    (
        0x861d70,
        &[
            0xf3, 0x0f, 0x11, 0x51, 0x0c, 0x66, 0xc7, 0x41, 0x10, 0x01, 0x00, 0x88, 0x51, 0x08,
            0xc3,
        ],
    ),
    // LoadingScreenData::reset: clear the request word.
    (
        0x861d80,
        &[
            0x33, 0xc0, 0xc6, 0x41, 0x08, 0x00, 0x89, 0x41, 0x0c, 0x66, 0x89, 0x41, 0x10,
        ],
    ),
    // Menu update checks menu+0x730, then passes menu+0x720 to the loading UI.
    (
        0x767e4f,
        &[0x80, 0xbb, 0x30, 0x07, 0x00, 0x00, 0x00, 0x74, 0x7a],
    ),
    (0x767e6a, &[0x4c, 0x8d, 0x83, 0x20, 0x07, 0x00, 0x00]),
];

#[cfg(test)]
fn fingerprints_match(image: &[u8]) -> bool {
    fingerprints_match_for_version(image, false)
}

fn fingerprints_match_for_version(image: &[u8], ww272: bool) -> bool {
    FINGERPRINTS.iter().all(|&(rva, bytes)| {
        let rva = if ww272 {
            match rva {
                0x861d70 => 0x862220,
                0x861d80 => 0x862230,
                0x767e4f => 0x7682ff,
                0x767e6a => 0x76831a,
                _ => return false,
            }
        } else {
            rva
        };
        image.get(rva..rva + bytes.len()) == Some(bytes)
    })
}

static VERIFIED: LazyLock<bool> = LazyLock::new(|| {
    let program = Program::current();
    let Ok(resources) = program.resources() else {
        return false;
    };
    let Ok(info) = resources.version_info() else {
        return false;
    };
    let Some(fixed) = info.fixed() else {
        return false;
    };
    let Some(language) = info.translation().first() else {
        return false;
    };
    let v = fixed.dwProductVersion;
    language.lang_id & 0x03ff == 0x0009
        && matches!(
            [v.Major, v.Minor, v.Patch, v.Build],
            [2, 7, 1, 0] | [2, 7, 2, 0]
        )
        && std::mem::offset_of!(CSMenuManImp, loading_screen_data) == 0x720
        && fingerprints_match_for_version(program.image(), v.Patch == 2)
});

pub(crate) fn screen_active() -> Option<bool> {
    if !*VERIFIED {
        return None;
    }
    let menu = unsafe { CSMenuManImp::instance() }.ok()?;
    // The dependency does not name the request byte yet. Its helper's ec/ed
    // bytes are per-frame latches, not a loading-screen indicator. Retain no
    // singleton address across frames/world recreation.
    let request = unsafe {
        std::ptr::addr_of!(menu.loading_screen_data)
            .cast::<u8>()
            .add(REQUEST_OFFSET)
            .read_volatile()
    };
    Some(request != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspected_game_image_loading_direction_and_lifecycle_guards() {
        use pelite::pe64::{PeFile, PeView};
        let Some(path) = std::env::var_os("ER_GAME_EXE") else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let file = PeFile::from_bytes(&bytes).unwrap();
        let ww272 = file.file_header().TimeDateStamp == 0x6aabf7e7;
        if ww272 {
            assert!(eldenring::verify_ww272_file(&bytes));
        }
        let mut mapped = vec![0; file.optional_header().SizeOfImage as usize];
        let headers = file.optional_header().SizeOfHeaders as usize;
        mapped[..headers].copy_from_slice(&bytes[..headers]);
        for section in file.section_headers() {
            let raw = file.get_section_bytes(section).unwrap();
            let start = section.VirtualAddress as usize;
            mapped[start..start + raw.len()].copy_from_slice(raw);
        }
        assert!(fingerprints_match_for_version(&mapped, ww272));
        assert!(crate::received_direction_layout_verified(Program::Mapping(
            PeView::from_bytes(&mapped).unwrap()
        )));
        let sites: &[usize] = if ww272 {
            &[
                0x862220, 0x862230, 0x7682ff, 0x76831a, 0x445fd9, 0x3c6d5f, 0x45ed80, 0x410759,
            ]
        } else {
            &[
                0x861d70, 0x861d80, 0x767e4f, 0x767e6a, 0x445ef9, 0x3c6d5f, 0x45eca0, 0x4106c9,
            ]
        };
        for (i, &at) in sites.iter().enumerate() {
            mapped[at] ^= 1;
            let accepted = if i < 4 {
                fingerprints_match_for_version(&mapped, ww272)
            } else {
                crate::received_direction_layout_verified(Program::Mapping(
                    PeView::from_bytes(&mapped).unwrap(),
                ))
            };
            assert!(!accepted, "accepted changed guard {at:x}");
            mapped[at] ^= 1;
        }
        let groups = if ww272 {
            include_bytes!("../tests/fixtures/ww272-register-event-groups.bin").as_slice()
        } else {
            include_bytes!("../tests/fixtures/ww271-register-event-groups.bin").as_slice()
        };
        let at = if ww272 { 0x4948e0 } else { 0x494800 };
        assert_eq!(&mapped[at..at + groups.len()], groups);
    }

    #[test]
    fn native_loading_request_layout_matches_verified_fixture() {
        assert_eq!(
            std::mem::offset_of!(CSMenuManImp, loading_screen_data),
            0x720
        );
        let fixture = include_bytes!("../tests/fixtures/ww271-loading-request.bin");
        let mut image = vec![0; 0x861da0];
        let mut offset = 0;
        for &(rva, bytes) in FINGERPRINTS {
            image[rva..rva + bytes.len()].copy_from_slice(&fixture[offset..offset + bytes.len()]);
            offset += bytes.len();
        }
        assert_eq!(offset, fixture.len());
        assert!(fingerprints_match(&image));
        for &(rva, _) in FINGERPRINTS {
            image[rva] ^= 1;
            assert!(!fingerprints_match(&image));
            image[rva] ^= 1;
        }
        assert!(!fingerprints_match(&[]));
    }
}
