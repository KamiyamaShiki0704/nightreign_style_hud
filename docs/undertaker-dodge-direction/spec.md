# Undertaker Dodge Direction

Version: 1.0
Confirmed: 2026-09-05 (user selected `atkDir_forSfx` and approved the mapping).

## Behavior

- UDD-01: When an incoming attack triggers the existing enhanced Undertaker
  pre-hit dodge, the system shall preserve generic player request `880922`
  and use that attack's `atkDir_forSfx` value for one additional player output.
- UDD-02: Values 0 (Front), 1 (Top), 2 (Bottom), 3 (Left), 4 (Right) shall map
  to `880930`, `880931`, `880932`, `880933`, `880934` respectively. These values
  describe the attack parameter, not attacker position or an evasion vector.
- UDD-03: When the attack or its direction cannot be resolved, the system
  shall emit only `880922`. It shall not reuse a previous attack's direction.
- UDD-04: Before each request, the system shall clear previous request and
  direction outputs, then make the direction available before the generic
  request. Output duration shall be 0.5s. Cancel, natural expiry and role
  cleanup shall also clear these outputs.
- UDD-05: Duplicate contacts shall not replace the first queued attack. The
  existing 0.40m anticipation margin, 650ms hit suppression, 900ms rearm,
  internal enhanced-state plus `880921` gate and skill timings shall remain.
- UDD-06: The DLL shall not send animation events or choose dodge animations;
  HKS owns animation selection and request consumption. The extension shall
  not scan all attacks every frame or affect attacks against other characters.

## Semantic Probe Cases

| Case | Preconditions / Input | Expected Output |
| --- | --- | --- |
| Known directions | Enhanced skill, one accepted attack with each value 0..4 | `880922` and exactly the corresponding direction output |
| Attack source | Different NPC/player attack entries with the same ID | Each uses its own entry's direction, not the other entry |
| Missing direction | Unknown value or unavailable attack entry | `880922` only, no previous direction remains |
| First contact | Two contacts before the request is consumed | Only the first determines the queued direction |
| Repeat | A new accepted contact after the existing rearm interval | New direction replaces old, never two direction outputs |
| Cleanup | Cancel, expiry or role change with a pending request | No stale request/direction remains |
| Exclusion | Ordinary skill, no enhanced buff, or other recipient | No new directional request |

Automated checks cover deterministic routing, decoding, queue ownership and
existing gates. Human acceptance covers actual in-game attack data, HKS
response, SpEffect application/expiry and cleanup. Observe pulse expiry within
one game-update interval of 0.5s; use contacts at least 900ms apart for rearm.

Out of scope: world-relative attack bearing, dodge animation selection, active
Regulation/HKS edits, changes to collision prediction or other DLLs.
