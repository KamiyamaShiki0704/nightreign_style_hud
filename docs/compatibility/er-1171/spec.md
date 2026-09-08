# ER 1.17.1 Support

Version: 1.0
Confirmed: 2026-09-08; user approved adaptation following the concrete static
audit and authorized pushing the dependency fix only.

## Contract

- ER1171-01: On the verified worldwide ER 1.17.1 executable, the HUD shall
  recognize its version and select the audited native entry points.
- ER1171-02: The four previously supported executable versions shall remain
  supported with their previous entry points. Unverified regional/future
  versions shall not silently select the new worldwide mappings.
- ER1171-03: Existing HUD/gameplay behavior, SpEffect IDs and timing shall
  remain unchanged. No unrelated upstream subsystem changes are included.
- ER1171-04: Cargo shall use an exact, remotely retrievable dependency commit;
  no vendored fsrs tree or permanent local dependency override is permitted.
- ER1171-05: Delivery shall back up and replace only the requested main HUD
  DLL. No main-HUD repository push or release publication is authorized.

## Verification

| Input / Precondition | Expected Result |
| --- | --- |
| Verified new worldwide executable | Recognized; all audited entry points match exactly, zero tolerance |
| Each of four retained executable versions | Previous routing and mappings retained |
| Unknown version or unverified region/version pair | Rejected, not treated as verified worldwide version |
| Locked HUD build | Resolves the published exact commit; package tests pass |
| Deployment | Backup retained; built and deployed hashes exactly equal |

Human acceptance: launch the updated game with the DLL, then verify raycast
aiming, role HUD/gameplay and Undertaker pre-hit/direction outputs. Automated
tests/build/static mapping checks do not constitute in-game acceptance.
No new behavioral timing threshold or animation changes are introduced.
