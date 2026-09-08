# ER 1.17.1 Build Result

Date: 2026-09-08. Contract: [spec.md](spec.md), version 1.0.
HUD base: 17590359edb382f1098c2cb3161d28fb7a0cbb5c plus preserved working changes.
KB revision: 0333a2ab2077c824e257148439db5ecdf4765f90.
Dependency: 3c8c1d7633a99309fb004c9f894ea10b7967d0e0, published on
KamiyamaShiki0704/fromsoftware-rs branch codex/er-1171-support.

## Automatic Results

- ER1171-01: new-version test failed before fix; passes after. All 92 entries
  match [mapping evidence](../er-1.17.1-20260908.json); five changed.
- ER1171-02: old-version routing and unknown-region rejection pass.
  Dependency command: `cargo test --locked -p eldenring --lib` (32/32).
- ER1171-03: HUD `cargo test --quiet --locked --offline -p nightreign-style-hud`
  passes 45/45. No gameplay-source changes in this adaptation. Runtime pending.
- ER1171-04: exact published commit fetched; no unrelated lockfile upgrades.
  `cargo build --release --locked --offline -p nightreign-style-hud` passes.
- ER1171-05: x64/static-CRT checks pass; built and deployed hashes match.
  Target: `F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.dll`.
  SHA256: `78A05ABDE4485B7671A83BB533A785D48E5C9921A40EC058B594AA1186481BA7`.
  Backup beside target: `nightreign_style_hud.pre-er1171-20260908-231259.dll.bak`.

Upstream checked at e0948c8d64594b79785e9490877baacea5605a9d (Sep 2):
still lacks 2.7.1.0. No broad upstream merge or main HUD/release publication.
Formatting/diff checks pass. No UE/Recorder execution applies to this native DLL.
Human acceptance pending: startup, raycast aiming, role features and Undertaker
direction outputs. Full runtime layouts and JP 2.7.1.x remain unverified.
