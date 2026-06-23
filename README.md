# Nightreign Style HUD

Experimental Elden Ring / Nightreign-style HUD DLL built on top of `fromsoftware-rs`.

This is a trimmed workspace that keeps only the crates needed by
`examples/nightreign-style-hud`:

- `crates/shared`
- `crates/eldenring`
- `crates/debug`
- `examples/nightreign-style-hud`

## Build

```powershell
cargo build --package nightreign-style-hud --release --target x86_64-pc-windows-msvc
```

Output:

```text
target\x86_64-pc-windows-msvc\release\nightreign_style_hud.dll
```

## Runtime Overview

The DLL reads player and enemy state every frame, draws the custom HUD with
`hudhook`, and uses Speffects as the main bridge between parameter-side logic,
TAE/EzState events, and DLL-side runtime systems.

Most role actions are driven by short-lived Speffects on the player. The DLL
consumes those input Speffects, updates internal meters, and may apply output
Speffects back to the player or enemies.

## Role Selection

Apply exactly one role marker Speffect to the player:

| Role | Player marker |
| --- | ---: |
| Wylder | `880000` |
| Guardian | `880100` |
| Ironeye | `880200` |
| Duchess | `880300` |
| Raider | `880400` |
| Revenant | `880500` |
| Recluse | `880600` |
| Executor | `880700` |
| Scholar | `880800` |
| Undertaker | `880900` |

The role marker controls the HUD icons, cooldown rules, and role-specific
systems.

## Shared Input Speffects

Most input events can be sent in either of these forms:

```text
880000 + offset
role_marker + offset
```

The shared form is preferred for new work. The role-relative form is kept for
older parameter setups. Example: Duchess skill use can be triggered by `880011`
or `880311`.

| Offset | Shared ID | Meaning |
| ---: | ---: | --- |
| `11` | `880011` | Skill used / consume one skill charge |
| `12` | `880012` | Secondary skill use event, currently consumed for compatibility |
| `13` | role-relative only | Enhanced skill use, currently used by Undertaker |
| `14` | role-relative only | Skill cancel, currently used by Undertaker |
| `16` | role-relative only | Undertaker free ultimate trigger |
| `21` | `880021` | Charged ultimate used |
| `22` | `880022` | Uncharged / discounted ultimate used |
| `24` | role-relative only | Executor ultimate cancel |
| `26` | `880026` | Ultimate gain from kill, adds `5%` with short cooldown |
| `27` | `880027` | Ultimate gain from critical hit, adds `5%` with short cooldown |

Damage hits also charge ultimate internally through the damage hook:

```text
hit gain = 0.8% per accepted damage event, throttled to 0.1s
kill / critical gain = 5%
```

## Shared Output Speffects

The DLL applies these player Speffects while meters are ready:

| ID | Meaning |
| ---: | --- |
| `880001` | Skill charge 1 ready |
| `880002` | Skill charge 2 ready |
| `880005` | Ultimate ready |

Legacy ready outputs `+3` and `+4` are removed on role changes, but the current
runtime writes `880005` for ultimate readiness.

## Role Mechanics

### Wylder

- Marker: `880000`
- Skill: 2 charges, 8s per charge.
- Ultimate: 335s base cooldown, discounted use supported.
- Held-skill lock UI:
  - Apply `880030` to the player while the skill is being held/aimed.
  - The HUD uses `Wylder_Skill_Lock.png`.
  - Valid enemies and wall/ground ray hits are shown in the default color.
  - No valid target falls back to a red center reticle.
  - The reticle attaches to the selected enemy or ray-hit point and scales by
    distance: close targets are larger, far targets are smaller.

### Guardian

- Marker: `880100`
- Skill: 1 charge, 14s cooldown.
- Ultimate: 335s base cooldown.

### Ironeye

- Marker: `880200`
- Skill: 2 charges, 10s per charge.
- Ultimate: 335s base cooldown, discounted use supported.
- Weakness mark:
  - Apply `880217` to an enemy to create or refresh a weakness mark.
  - DLL consumes `880217` from the enemy.
  - DLL applies `880218` while the mark is active.
  - DLL applies `880219` when the mark breaks.
  - Mark lasts 17s.
  - Accumulated damage threshold is 20% of the enemy's max HP.
  - Damage bonus while marked is expected to be handled parameter-side.
- Precision aiming:
  - Native `SB_Reticle` is hidden.
  - Custom `Ironeye_Reticle1/2/3` drawing is used.
  - Camera/reticle uses a visual convergence scheme.

### Duchess

- Marker: `880300`
- Skill: 1 charge, 12s cooldown.
- Replay:
  - Trigger with `880011` or `880311`.
  - Scans enemies within 8m around the player.
  - Replays damage received by each enemy during the last 3.5s.
  - Replayed result is scaled to 50%.
  - Replays HP damage, status buildup, and posture/poise reduction where the
    data is available.
  - Runtime damage Speffect: `880365`.
- Visual note:
  - The real ghost/replay visual is still TODO. Speffect tinting and spawned
    debug pawns were intentionally not kept because they did not match the
    desired bloodstain-style phantom behavior or had cleanup risks.

### Raider

- Marker: `880400`
- Skill: 1 charge, 12s cooldown.
- Ultimate: 335s base cooldown.

### Revenant

- Marker: `880500`
- Skill HUD has 3 family HP rows.
- Family summon triggers:
  - `270000` = family slot 1
  - `271000` = family slot 2
  - `272000` = family slot 3
- BuddyParam rows expected by the DLL:
  - `27000000`
  - `27100000`
  - `27200000`
- Only one family member is active at a time.
- Triggering a different family while one is active follows the native Buddy
  behavior: the current family is recalled first; trigger again to summon the
  next one.
- Family HP is remembered across recall/resummon.
- Recalled family members regenerate 2% HP per second.
- Summon range is patched for Revenant so Buddy summon range is effectively
  expanded to 1000.
- Passive conversion:
  - On Revenant kill, 15% chance to convert the killed enemy into a friendly
    entity.
  - Max converted souls: 10.
  - Lifetime: 60s, or until death.
  - Converted soul Speffect: `295000`.
  - Converted soul team type: `47`.
  - Appear animation: `1830`.
  - Disappear animation: `1840`.
  - Current implementation uses the entity route; the older passive Buddy route
    is disabled.

### Recluse

- Marker: `880600`
- Skill has no cooldown meter; it is available while Recluse is active.
- Absorb / release flow:
  - Use skill with `880011` or `880611` while a valid locked target has an
    elemental damage trace.
  - Release/clear stored magic with `880618`.
- Output Speffects on player:
  - `880619` = restore FP pulse.
  - `880651` = absorbed magic.
  - `880652` = absorbed fire.
  - `880653` = absorbed lightning.
  - `880654` = absorbed holy.
  - `880631..880644` = mixed magic result slots 1..14.
- Stored element UI is drawn by the DLL.
- Locked target elemental trace is cleared when absorbed.

### Executor

- Marker: `880700`
- Skill is no-cooldown in the HUD layer.
- Ultimate transform:
  - Trigger charged ultimate with `880021` or `880721`.
  - Transform drains for 15s.
  - Cancel with `880724`.
  - When auto-ended, DLL applies `880725`.
  - During transform, normal hit/kill/critical ultimate gain is ignored.

### Scholar

- Marker: `880800`
- Skill: 1 charge, 12s cooldown.
- Observation:
  - Hold/apply `880830` to enter observe mode.
  - Scans multiple visible enemies inside the lens.
  - Filters invalid/hidden/dead targets, `c1000`, distance, screen position,
    lock volume, and line of sight.
  - Observation progress decays when targets are not visible.
- Observation apply events:
  - `880831` applies a self buff based on the highest observed target.
  - `880832` applies enemy debuffs based on each observed target.
  - Applying either effect clears the observation gauges.
- Scholar output Speffects:
  - Self stages: `880846..880850`.
  - Enemy stages: `880841..880845`.
- Sympathy links:
  - `880860` = Scholar/self link.
  - `880861` = ally link.
  - `880862` = enemy link.
  - Damage to linked enemies can spread damage and heal allies.
  - Healing on linked allies/self can convert to enemy damage and spread healing.
  - Scholar being hit by a linked enemy can reflect damage.
  - Native damage-number compatible damage uses `880865` with runtime-modified
    Speffect HP delta.

### Undertaker

- Marker: `880900`
- Skill:
  - Normal use: `880011` or `880911`.
  - Enhanced use: `880913`.
  - Cancel: `880914`.
  - Normal duration: 8s.
  - Enhanced duration: 15s.
  - When the skill duration auto-ends, DLL applies `880915`.
- Ultimate:
  - Discounted use supported.
  - Free ultimate trigger: `880916`.
  - Free ultimate window: 4s.

## Independent Systems

### Camera-Directed Movement

- Activation Speffect: `880951` on the player.
- This is role-independent.
- The runtime hook blends root-motion direction with camera pitch/yaw for
  selected animations. Current tuned behavior emphasizes vertical influence
  without allowing the earlier extreme launch behavior.

### Multi-Lock

Multi-lock is role-independent.

Player activation Speffects are limited to:

```text
882000..882099
```

Each UI style uses a group of 5 IDs:

| Group offset | Target limit |
| ---: | ---: |
| `0` | 4 |
| `1` | 8 |
| `2` | 16 |
| `3` | 32 |
| `4` | reserved / ignored |

Examples:

| Player Speffect | Meaning |
| ---: | --- |
| `882000` | 4 targets, style 0 |
| `882001` | 8 targets, style 0 |
| `882002` | 16 targets, style 0 |
| `882003` | 32 targets, style 0 |
| `882005` | 4 targets, style 1 |
| `882006` | 8 targets, style 1 |
| `882007` | 16 targets, style 1 |
| `882008` | 32 targets, style 1 |

Selected enemies receive marker Speffects:

```text
882100 + style_index
```

Examples:

```text
882000..882003 -> enemies receive 882100
882005..882008 -> enemies receive 882101
882010..882013 -> enemies receive 882102
```

Behavior:

- Target collection reuses Scholar-style screen/lens/range/line-of-sight logic.
- Built-in `MultipleLocking.png` target rendering is disabled.
- Visible target UI should be implemented parameter/VFX-side through the enemy
  marker Speffects.
- Spawn-hook bullets can be duplicated toward multiple targets.
- Live weapon bullets are also scanned and guided.
- Current live weapon guidance mix: `0.4`.

## Assets

HUD assets live in:

```text
examples\nightreign-style-hud\assets
```

Several PNGs were combined into atlases to reduce runtime texture pressure. The
visual layout should remain equivalent to the previous individual-PNG setup.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-ASL2](LICENSE-ASL2))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Notes And Known Limits

- The native summon HUD is not fully disabled yet; Revenant uses the custom HUD
  alongside the remaining native Buddy UI.
- Duchess replay phantom visuals are still TODO.
- Scholar ally-reflect coverage is stable for Scholar-self reflection; full
  teammate-hit reflection can still be expanded.
- Generated binaries and atlas previews are ignored by `.gitignore`.
