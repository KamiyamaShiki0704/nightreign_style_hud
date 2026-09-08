# Precision Aim 1.17.1 Build

Date: 2026-09-08. Contract: precision-spec.md@1.0, AIM1171-01..03.
Workspace base: 17590359edb382f1098c2cb3161d28fb7a0cbb5c plus release changes.
KB: 0333a2ab2077c824e257148439db5ecdf4765f90.
Dependency: 3c8c1d7633a99309fb004c9f894ea10b7967d0e0.

The standalone crate has no private fixed RVA: audited shared task registration
now selects EB3E50 on WW 2.7.1.0. No runtime camera or asset logic changed.
Added three pure checks for activation/asset names, embedded PNG decoding and
projection. `cargo test --locked --offline -p nightreign-precision-aim`: 3/3.
Main HUD tests: 45/45. Locked offline release build of both packages passes.
PE machine 0x8664; no dynamic MSVC CRT normal/delay imports in either DLL.

Main SHA256: 78a05abde4485b7671a83bb533a785d48e5c9921a40ec058b594aa1186481ba7.
Aim SHA256: 2d2bcefb9a231f7a0292b35054e884503bf46b1dc8a2f93e133a8980430dfc0f.
Release tag v0.3.2 binds the final source revision and separate download packages.
No new relative-bearing feature is included in that release.
Manual probes from precision-spec.md remain pending; no in-game PASS claimed.
