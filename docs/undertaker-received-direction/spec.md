# Undertaker Received Damage Bearing

Version: 1.0. Confirmed by user on 2026-09-08.

- UDB-01: When an armed enhanced-skill auto-dodge captures the first incoming
  hit, it shall latch that hit's native player-relative received direction
  at contact time, before damage effects, without playing an animation.
- UDB-02: A verified front/back/left/right result shall add exactly one player
  output: 880935/880936/880937/880938 respectively, before generic 880922.
  Unverified/missing direction shall emit no bearing marker, never stale data.
- UDB-03: Existing Atk type markers 880930..880934, 880922, eligibility,
  cooldown and invulnerability behavior shall remain unchanged.
- UDB-04: Bearing markers shall last 0.5s through supplied Param rows and be
  cleared on the next trigger or enhanced-skill exit. Simultaneous later
  hits or subsequent character rotation shall not replace the latched result.
- UDB-05: No actual received damage, hit sound or reaction shall be required
  to obtain direction. No native HKS getter override is requested for this step.

## Probes

Precondition: enhanced skill armed, player alive, no trigger pending/cooldown.
Input: separate attacks from player-relative front/back/left/right.
Expected: 880922 plus only matching bearing marker, and existing Atk marker
when known; no HP loss or received-hit sound/reaction. Direction comparison
must follow verified native classification, with exact enum equality.
Repeat with projectiles, simultaneous contacts and rotation after capture.
Unknown/invalid data: no new bearing marker, existing trigger still works.
Removal/rearm: old markers absent on next request or exit; supplied duration
0.5s. Existing sensor margin 0.40m, suppression 650ms and rearm 900ms unchanged.
Automatic tests cover mapping, invalid data, latching, cleanup membership
and pure native-equivalent classification where evidenced.
Human acceptance covers actual attack/projectile directions, audio, damage,
timing and exit transitions. No attacker-position-only approximation, damage
execution, HKS edits, normal-skill expansion or diagnostic HUD is in scope.
