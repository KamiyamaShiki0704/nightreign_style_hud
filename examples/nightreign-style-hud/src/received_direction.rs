static UNDERTAKER_RECEIVED_DIRECTION_AVAILABLE: AtomicBool = AtomicBool::new(false);

const UNDERTAKER_PENDING_ATK_MASK: u64 = (1 << 33) - 1;
const UNDERTAKER_PENDING_BEARING_SHIFT: u32 = 33;
const UNDERTAKER_PENDING_BEARING_MASK: u64 = 7 << UNDERTAKER_PENDING_BEARING_SHIFT;
const RECEIVED_DIRECTION_COS_45: f32 = f32::from_bits(0x3f3504f3);
// Native HKS enums: left=0, right=1, front=2, back=3.
const SP_UNDERTAKER_RECEIVED_DIRECTIONS: [i32; 4] = [880937, 880938, 880935, 880936];

fn received_direction_version_supported(lang: u16, version: [u16; 4]) -> bool {
    lang & 0x03ff == 0x0009 && matches!(version, [2, 7, 1, 0] | [2, 7, 2, 0])
}

fn received_direction_layout_verified(program: Program) -> bool {
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
    if !received_direction_version_supported(language.lang_id, [v.Major, v.Minor, v.Patch, v.Build])
    {
        return false;
    }
    let image = program.image();
    // Version-scoped consumers of the incoming vector, interpolation quaternion
    // and HKS pair. Never enable the new raw reads on an unverified layout.
    [
        (0x445ef9, &[0x48, 0x8d, 0x9e, 0xc0, 0x01, 0x00, 0x00][..]),
        (
            0x3c6d5f,
            &[
                0x48, 0x8b, 0x41, 0x10, 0x48, 0x8b, 0xda, 0x48, 0x8b, 0x88, 0x90, 0x01, 0x00, 0x00,
                0x48, 0x8b, 0x49, 0x68,
            ][..],
        ),
        (0x45eca0, &[0x48, 0x8d, 0x41, 0x60, 0xc3][..]),
        (0x4106c9, &[0xf2, 0x0f, 0x10, 0x41, 0x38][..]),
    ]
    .into_iter()
    .all(|(rva, expected)| {
        let rva = if v.Patch == 2 {
            match rva {
                0x445ef9 => 0x445fd9,
                0x3c6d5f => 0x3c6d5f,
                0x45eca0 => 0x45ed80,
                0x4106c9 => 0x410759,
                _ => return false,
            }
        } else { rva };
        image.get(rva..rva + expected.len()) == Some(expected)
    })
}

fn capture_undertaker_received_direction(damage_module: usize, payload: usize) -> Option<u8> {
    if !UNDERTAKER_RECEIVED_DIRECTION_AVAILABLE.load(Ordering::Acquire)
        || damage_module == 0
        || payload == 0
    {
        return None;
    }
    // All pointers belong to this live native dispatch. Copy scalar inputs only;
    // no reference or pointer is retained across the pending-hit handoff.
    let (rotation, incoming) = unsafe {
        let read_usize = |address: usize| std::ptr::read_unaligned(address as *const usize);
        let owner = read_usize(damage_module + 0x08);
        if owner == 0 {
            return None;
        }
        let proxy = read_usize(owner + 0x58);
        if proxy == 0 {
            return None;
        }
        let chr = read_usize(proxy + 0x10);
        if chr == 0 {
            return None;
        }
        let bag = read_usize(chr + 0x190);
        if bag == 0 {
            return None;
        }
        let physics = read_usize(bag + 0x68);
        if physics == 0 {
            return None;
        }
        (
            std::ptr::read_unaligned((physics + 0x60) as *const [f32; 4]),
            std::ptr::read_unaligned((payload + 0x1c0) as *const [f32; 4]),
        )
    };
    received_direction_from_snapshot(rotation, incoming)
}

fn received_direction_from_snapshot(q: [f32; 4], incoming: [f32; 4]) -> Option<u8> {
    if !q.iter().chain(incoming[..3].iter()).all(|v| v.is_finite()) || q.iter().all(|&v| v == 0.0) {
        return None;
    }
    let [x, y, z, w] = q;
    // Match 3C6D40's quaternion matrix applied to -Z, including operation order.
    let facing = [
        -(w * (y + y) + x * (z + z)),
        -((1.0 - x * (x + x)) - y * (y + y)),
    ];
    let incoming = [-incoming[0], -incoming[2]];
    let normalize = |v: [f32; 2]| {
        let length_squared = v[0] * v[0] + v[1] * v[1];
        if !length_squared.is_finite() || length_squared <= 0.0 {
            return None;
        }
        let length = length_squared.sqrt();
        Some([v[0] / length, v[1] / length])
    };
    let f = normalize(facing)?;
    let v = normalize(incoming)?;
    let dot = (f[0] * v[0] + f[1] * v[1]).clamp(-1.0, 1.0);
    let cross_y = f[1] * v[0] - f[0] * v[1];
    classify_received_direction(dot, cross_y)
}

fn classify_received_direction(dot: f32, cross_y: f32) -> Option<u8> {
    if !dot.is_finite() || !cross_y.is_finite() || !(-1.0..=1.0).contains(&dot) {
        return None;
    }
    Some(if dot >= RECEIVED_DIRECTION_COS_45 {
        2
    } else if dot >= -RECEIVED_DIRECTION_COS_45 {
        if cross_y >= 0.0 { 1 } else { 0 }
    } else {
        3
    })
}

fn pending_hit_with_received_direction(attack: u64, bearing: Option<u8>) -> u64 {
    if attack == 0 {
        return 0;
    }
    let bearing = bearing
        .filter(|&value| value < 4)
        .map_or(0, |value| u64::from(value) + 1);
    (attack & UNDERTAKER_PENDING_ATK_MASK) | (bearing << UNDERTAKER_PENDING_BEARING_SHIFT)
}

fn pending_received_direction_effect(pending: u64) -> Option<i32> {
    if pending & !(UNDERTAKER_PENDING_ATK_MASK | UNDERTAKER_PENDING_BEARING_MASK) != 0
        || pending & 3 == 0
    {
        return None;
    }
    let tag =
        ((pending & UNDERTAKER_PENDING_BEARING_MASK) >> UNDERTAKER_PENDING_BEARING_SHIFT) as usize;
    tag.checked_sub(1)
        .and_then(|index| SP_UNDERTAKER_RECEIVED_DIRECTIONS.get(index).copied())
}

fn undertaker_direction_cleanup_effects() -> impl Iterator<Item = i32> {
    SP_UNDERTAKER_AUTO_DODGE_DIRECTIONS
        .into_iter()
        .chain(SP_UNDERTAKER_RECEIVED_DIRECTIONS)
}

#[cfg(test)]
mod received_direction_tests {
    use super::*;

    #[test]
    fn matches_all_isolated_native_rotation_and_boundary_samples() {
        let fixture = include_str!("../tests/fixtures/received-direction-1171.tsv");
        let mut count = 0;
        for (line_number, line) in fixture.lines().enumerate().skip(1) {
            let cells: Vec<_> = line.split('\t').collect();
            assert_eq!(cells.len(), 9);
            let mut values = [0.0; 8];
            for (value, cell) in values.iter_mut().zip(&cells) {
                *value = cell.parse::<f32>().unwrap();
            }
            let q = values[..4].try_into().unwrap();
            let v = values[4..].try_into().unwrap();
            assert_eq!(
                received_direction_from_snapshot(q, v),
                Some(cells[8].parse::<u8>().unwrap()),
                "native sample {}",
                line_number + 1
            );
            count += 1;
        }
        assert_eq!(count, 1176);
    }

    #[test]
    fn both_direction_markers_precede_generic_and_missing_atk_keeps_bearing() {
        let pending =
            pending_hit_with_received_direction(pack_undertaker_pending_attack(100, 1), Some(0));
        assert_eq!(
            undertaker_dodge_effects(pending, |_, _| Some(1)),
            [Some(880931), Some(880937), Some(880922)]
        );
        let unknown =
            pending_hit_with_received_direction(UNDERTAKER_PENDING_UNKNOWN_ATTACK, Some(3));
        assert_eq!(
            undertaker_dodge_effects(unknown, |_, _| panic!("invalid Atk must not be resolved")),
            [None, Some(880936), Some(880922)]
        );
    }

    #[test]
    fn native_cardinal_and_rotated_snapshots_match() {
        let identity = [0.0, 0.0, 0.0, 1.0];
        for (incoming, expected) in [
            ([0.0, 0.0, 1.0, 0.0], 2),
            ([0.0, 0.0, -1.0, 0.0], 3),
            ([1.0, 0.0, 0.0, 0.0], 1),
            ([-1.0, 0.0, 0.0, 0.0], 0),
        ] {
            assert_eq!(
                received_direction_from_snapshot(identity, incoming),
                Some(expected)
            );
        }
        let c = RECEIVED_DIRECTION_COS_45;
        assert_eq!(
            received_direction_from_snapshot([0.0, c, 0.0, c], [1.0, 4.0, 0.0, 0.0]),
            Some(2)
        );
        assert_eq!(
            received_direction_from_snapshot([0.0, -c, 0.0, c], [-1.0, 0.0, 0.0, 0.0]),
            Some(2)
        );
        assert_eq!(
            received_direction_from_snapshot([0.0, 1.0, 0.0, 0.0], [0.0, 0.0, -1.0, 0.0]),
            Some(2)
        );
    }

    #[test]
    fn native_classifier_boundaries_and_tie_priority_are_exact() {
        let c = RECEIVED_DIRECTION_COS_45;
        assert_eq!(classify_received_direction(c, -1.0), Some(2));
        assert_eq!(
            classify_received_direction(f32::from_bits(c.to_bits() - 1), -1.0),
            Some(0)
        );
        assert_eq!(classify_received_direction(-c, 1.0), Some(1));
        assert_eq!(classify_received_direction(-c, -1.0), Some(0));
        assert_eq!(
            classify_received_direction(f32::from_bits((-c).to_bits() + 1), 1.0),
            Some(3)
        );
        assert_eq!(classify_received_direction(0.0, 0.0), Some(1));
    }

    #[test]
    fn invalid_or_degenerate_input_never_becomes_a_direction() {
        let q = [0.0, 0.0, 0.0, 1.0];
        for v in [
            [0.0; 4],
            [0.0, 1.0, 0.0, 0.0],
            [f32::NAN, 0.0, 1.0, 0.0],
            [f32::MAX, 0.0, f32::MAX, 0.0],
        ] {
            assert_eq!(received_direction_from_snapshot(q, v), None);
        }
        assert_eq!(
            received_direction_from_snapshot([0.0; 4], [1.0, 0.0, 0.0, 0.0]),
            None
        );
        assert_eq!(classify_received_direction(f32::NAN, 0.0), None);
        assert_eq!(classify_received_direction(0.0, f32::INFINITY), None);
    }

    #[test]
    fn bearing_is_published_atomically_with_the_original_attack() {
        let pending = AtomicU64::new(0);
        let first = pending_hit_with_received_direction(
            pack_undertaker_pending_attack(i32::MAX, 1),
            Some(2),
        );
        let second =
            pending_hit_with_received_direction(pack_undertaker_pending_attack(1, 0), Some(3));
        assert_eq!(
            pending.compare_exchange(0, first, Ordering::AcqRel, Ordering::Acquire),
            Ok(0)
        );
        assert_eq!(
            pending.compare_exchange(0, second, Ordering::AcqRel, Ordering::Acquire),
            Err(first)
        );
        let taken = pending.swap(0, Ordering::AcqRel);
        assert_eq!(
            unpack_undertaker_pending_attack(taken),
            Some((1, i32::MAX as u32))
        );
        assert_eq!(pending_received_direction_effect(taken), Some(880935));
        assert_eq!(pending.swap(0, Ordering::AcqRel), 0);
    }

    #[test]
    fn received_marker_mapping_unknown_keys_and_cleanup_are_independent() {
        for (raw, sp) in [(0, 880937), (1, 880938), (2, 880935), (3, 880936)] {
            let value =
                pending_hit_with_received_direction(UNDERTAKER_PENDING_UNKNOWN_ATTACK, Some(raw));
            assert_eq!(pending_received_direction_effect(value), Some(sp));
        }
        for raw in [None, Some(4), Some(255)] {
            assert_eq!(
                pending_received_direction_effect(pending_hit_with_received_direction(1, raw)),
                None
            );
        }
        assert_eq!(pending_hit_with_received_direction(0, Some(2)), 0);
        assert_eq!(pending_received_direction_effect(u64::MAX), None);
        let cleanup: Vec<_> = undertaker_direction_cleanup_effects().collect();
        assert_eq!(cleanup.len(), 9);
        for sp in 880930..=880938 {
            assert!(cleanup.contains(&sp));
        }
    }

    #[test]
    fn new_raw_input_layout_is_scoped_to_verified_worldwide_1171() {
        assert!(received_direction_version_supported(0x0409, [2, 7, 1, 0]));
        assert!(!received_direction_version_supported(0x0411, [2, 7, 1, 0]));
        assert!(!received_direction_version_supported(0x0409, [2, 7, 0, 0]));
        assert!(received_direction_version_supported(0x0409, [2, 7, 2, 0]));
        assert!(!received_direction_version_supported(0x0409, [2, 7, 3, 0]));
    }
}
