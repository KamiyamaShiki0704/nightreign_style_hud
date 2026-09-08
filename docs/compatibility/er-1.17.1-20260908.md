# ER 1.17.1 Static Compatibility Audit

Date: 2026-09-08. Target: `nightreign_style_hud`; no gameplay changes made.
Machine-readable evidence: [audit JSON](er-1.17.1-20260908.json).

## Verdict

Adaptation is required. The deployed DLL's pinned fsrs dependency does not
recognize WW product version `2.7.1.0`. In addition, five bundled native RVAs
moved; adding a version alias to the old bundle is not sufficient.

Input EXE: `F:\SteamLibrary\steamapps\common\ELDEN RING\Game\eldenring.exe`,
87042128 bytes, product/file version 2.7.1.0. SHA-256:
`1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.

Dependency: `KamiyamaShiki0704/fromsoftware-rs` at
`f96aa78e26d51838c31c411e05a41834631d54d5`.
Its `crates/eldenring/src/rva.rs` only recognizes WW 2.6.2.0/2.7.0.0 and
JP 2.6.2.1/2.7.0.1. `rva::get()` panics on an unsupported version; the HUD
release profile uses `panic = "abort"`. Do not load the old DLL expecting
safe fallback. No crash was deliberately induced during this audit.

## Address Results

Built the pinned dependency's existing binary-mapper in this project's
ignored `target/compat-audit` directory. Ran its read-only `map` command with
the pinned ER mapper profile against the new EXE (exit 0). Compared all 92
outputs to `RVAS_2700`: 87 identical, 5 changed, none unresolved/zero.

| Mapping | 2.7.0.0 RVA | 2.7.1.0 RVA | Delta |
| --- | --- | --- | --- |
| `register_task` | `0xEB3DE0` | `0xEB3E50` | `+0x70` |
| `cs_phys_world_cast_ray` | `0xC71D90` | `0xC71E00` | `+0x70` |
| `cs_ez_draw_draw_line` | `0x26525D0` | `0x2652640` | `+0x70` |
| `cs_ez_draw_draw_triangle` | `0x2652320` | `0x2652390` | `+0x70` |
| `cs_ez_draw_draw_wedge` | `0x2651ED0` | `0x2651F40` | `+0x70` |

The main HUD uses task registration for recurring gameplay/camera processing,
and native raycasts for camera/aiming/line-of-sight work. Disassembly confirms
the new entries; old `0xEB3DE0` now points to an unrelated getter and old
`0xC71D90` is not the raycast entry. Never shift the entire bundle by 0x70:
the other 87 outputs remain at their old addresses.

Selected unchanged mappings: Bullet spawn `0x3A2CB0`, apply SpEffect
`0x3E8DC0`, remove SpEffect `0x3EE2E0`, PlayerIns VMT `0x2A7FBB0`,
EnemyIns VMT `0x2A47090`, GameMan `0x3D6D988`, GameDataMan `0x3D61F98`.
The HUD's current layout selector compares the Bullet spawn RVA to
`0x3A2CB0`, so it will still select the 1.17 layout once version support is
added. This observation alone does not prove every object field is unchanged.

## Main HUD Hooks And Atk Payload

Scanning the PE code range with the exact four current source patterns found
one match for each:

| Pattern | New EXE match |
| --- | --- |
| Damage process | `0x448910` |
| Undertaker pre-hit dispatch | `0x449D30` |
| Revenant summon range | `0xEA1577` |
| Camera-directed position | `0x467DF9` |

No source-pattern replacements are indicated by these results. Damage and
Undertaker dispatch remain at their recorded previous RVAs. The pattern-based
Revenant hook resolves its current site rather than calling a hardcoded RVA.

At `0x449DDB/0x449DE4`, the current executable still reads incoming attack
table from payload `+0x44` and row ID from `+0x40`. At `0x449E24` the character
module bag is still accessed at `+0x190`. The call at `0x449DFD` now targets
Atk resolver `0xD247D0` (previously `0xD24760`); that resolver still selects
repository slot `table + 7`, with NPC=0 and PC=1. The HUD does not call that
resolver by a fixed address: its direction feature uses the typed repository.

## Remaining Adaptation

1. Add a specifically verified WW 2.7.1.0 version branch and separate bundle
   containing the five corrected RVAs, retaining previous version support.
2. Add version/routing tests. Do not invent new JP mappings without its EXE.
3. Commit the dependency fix in its own project, then update this project's
   Cargo git revision/lockfile to that exact commit, following the user's
   preference for pinned Git dependencies rather than a vendored fsrs tree.
4. Rebuild and test the requested DLLs. Prioritize startup/task registration,
   raycast-backed aim systems and Undertaker pre-hit/direction outputs.

This was a static audit, not an in-game acceptance run. No full old/new EXE
or object-layout diff was available; HP/FP, camera, Buddy and other runtime
fields must not be claimed exhaustively verified from VMT/pattern stability.
Other DLL projects were not individually audited in this task. A DLL using
the same unsupported dependency version has the same version-gate risk.

The deployed main DLL was not changed. Its SHA-256 remains
`F18300ED4994A039900FB5100B68D1E6537970E4A9A68CAF0B55A781AB57E564`.
No gameplay DLL rebuild, deployment, commit, push or release was performed.
