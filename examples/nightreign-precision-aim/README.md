# Nightreign Precision Aim

Standalone DLL for the Nightreign/Ironeye-style bow precision-aim camera and
custom reticle.

## Game Compatibility

This build uses the workspace's pinned `fromsoftware-rs` compatibility commit:

```text
3c8c1d7633a99309fb004c9f894ea10b7967d0e0
```

The binding layer explicitly supports Elden Ring `2.6.2.0`, `2.7.0.0` and
`2.7.1.0` (Worldwide), plus `2.6.2.1` and `2.7.0.1` (Japanese).
Worldwide `2.7.1.0` is patch 1.17.1; new Japanese 2.7.1.x is not verified.
The standalone crate contains no fixed game RVA of its
own; singleton and task addresses are selected by that versioned binding
layer. Unknown executable versions are rejected rather than using guessed
addresses.

The 1.17.1 build includes the corrected shared task-registration address;
activation, camera tuning and external reticle loading remain unchanged.
This DLL and the main HUD are distributed on the same repository's Releases
page, in separate packages. In-game validation is still required.

## Activation

Apply Speffect `883100` to the local player. While the player is manually aiming
or precision shooting, the DLL:

- suppresses the native precision reticle/zoom timing,
- keeps projectile precision behavior active around projectile and damage tasks,
- applies the same visual camera tuning used by `nightreign_style_hud`,
- draws the custom reticle using a fixed convergence distance.

The DLL is independent of role marker Speffects.

## Custom Reticle PNGs

The reticle uses three PNG layers:

- `Precision_Reticle1.png`
- `Precision_Reticle2.png`
- `Precision_Reticle3.png`

At runtime the DLL tries to load these from an `assets` folder next to the DLL.
For each layer it falls back to the matching `Ironeye_ReticleN.png`, and then
to the embedded default compiled into the DLL if both external files are
missing or invalid.

For customization, keep the DLL and the `assets` folder together, then replace
the three `Precision_ReticleN.png` files. Square 256x256 PNGs are recommended.

## Installation

Keep the DLL and its private assets together:

```text
nightreign_precision_aim/
|-- nightreign_precision_aim.dll
`-- assets/
    |-- Precision_Reticle1.png
    |-- Precision_Reticle2.png
    |-- Precision_Reticle3.png
    |-- Ironeye_Reticle1.png
    |-- Ironeye_Reticle2.png
    `-- Ironeye_Reticle3.png
```

The `Ironeye_` files are packaged fallback/default layers. User overrides
should normally use the `Precision_` names.

## Coexistence with Nightreign Style HUD

`nightreign_style_hud.dll` already provides the same `883100` precision-aim
runtime. When the full HUD DLL is present, this standalone DLL deliberately
does not initialize a second task/ImGui runtime; the full HUD handles the
feature instead. Disable the full HUD when validating this DLL by itself and
when testing this folder's private reticle assets.

## Build

```powershell
cargo build --package nightreign-precision-aim --release --target x86_64-pc-windows-msvc --locked
```

Build output:

```text
target\x86_64-pc-windows-msvc\release\nightreign_precision_aim.dll
```

The workspace target configuration links the Microsoft C runtime statically,
so the release DLL does not require a separately installed dynamic MSVC CRT.
