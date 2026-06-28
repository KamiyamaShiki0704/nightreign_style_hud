# Nightreign Precision Aim

Standalone DLL for the Ironeye-style precision aiming camera and reticle.

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

- `Ironeye_Reticle1.png`
- `Ironeye_Reticle2.png`
- `Ironeye_Reticle3.png`

At runtime the DLL tries to load these from an `assets` folder next to the DLL.
If a PNG is missing or invalid, the embedded default compiled into the DLL is
used for that layer.

For customization, keep the DLL and the `assets` folder together, then replace
the three PNGs with files of the same names. Square 256x256 PNGs are recommended.

## Build

```powershell
cargo build --package nightreign-precision-aim --release --target x86_64-pc-windows-msvc
```

Build output:

```text
target\x86_64-pc-windows-msvc\release\nightreign_precision_aim.dll
```
