# Received Bearing Build

Date: 2026-09-09. Input: spec.md@1.0, UDB-01..05; design.md.
HUD base: 215a2158bf32e5027dc9167368ca68bcee705368 plus local direction changes.
KB: 9cff53bb208e0be3d39acfeffb224387224ca59a.
fsrs: 3c8c1d7633a99309fb004c9f894ea10b7967d0e0 (unchanged).

- UDB-01/02: version/fingerprint-gated input snapshot and native-equivalent
  pure classifier; fixture1176/1176 matches actual Rust, including optimized build.
- UDB-03/04: atomic key+bearing first-contact tests, output ordering and cleanup
  membership pass; original eligibility/cooldown untouched. Four CSV rows are
  neutral clones with0.5s duration; all other fields match existing markers.
- UDB-05: no call to damage or animation, no env222 write/override, no debug HUD.
- `cargo test --locked --offline -p nightreign-style-hud`:53/53 pass.
- `cargo test --release --locked --offline -p nightreign-style-hud received_direction_tests`:8/8 pass.
- Locked offline release, formatting/diff, x64/static CRT checks pass.
- User confirmed game closed; old DLL backed up and deployed hash matches build:
  `0EE756B2339044DC0B281A21BC8B42B6AF744F38FF1774A81EF2EA46FD61B742`.
  Target: `F:\GoldenAge\_GoldenAge\dll\nightreign\nightreign_style_hud.dll`.
  Backup beside target: `nightreign_style_hud.pre-received-direction-20260909-000840.dll.bak`.

Evidence: native-research.md and tests/fixtures/received-direction-1171.*
under examples/nightreign-style-hud. Spec probes requiring live game acceptance
remain pending: physical sides/projectiles, no HP/audio/reaction, rotation,
duplicates and exit behavior. No UE/Recorder run applies to this native DLL.
Local development only; compatibility releasev0.3.2 still contains the earlier
main DLL and updated standalone aim. Do not conflate published and local hashes.
