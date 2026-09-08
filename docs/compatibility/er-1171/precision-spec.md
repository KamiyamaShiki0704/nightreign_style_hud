# Precision Aim 1.17.1 Delivery

Version: 1.0. User confirmed update and common publication on 2026-09-08.

- AIM1171-01: Standalone aiming shall support the verified WW 1.17.1 game
  using the same audited dependency revision as the main HUD.
- AIM1171-02: Activation 883100, camera settings, private assets and coexistence
  behavior shall remain unchanged; no other standalone DLL is in scope.
- AIM1171-03: Both builds shall be downloadable from the same main HUD
  repository release, with source revision, checksums and rollback retained.

Automatic checks: locked build/test, exact dependency revision, valid PNGs,
x64/static CRT imports, archive content/hash match and remote asset digest.
Human checks: standalone startup and 883100 manual aim; removal restores
camera; custom PNG loading; main-HUD coexistence without duplicate hooks.
No new timing/geometry thresholds. Native runtime behavior remains a human
acceptance item, not inferred from successful compilation.
