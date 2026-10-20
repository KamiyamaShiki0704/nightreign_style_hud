# Received Bearing Design

Input: spec.md@1.0, UDB-01..05. Native evidence: native-research.md.
Scope: only main HUD; no native HKS override and no dependency edits.

WW 1.17.1 dispatcher 449D30 has target damage module in RCX, payload in R8.
Before rejecting, snapshot payload vector +1C0 and target interpolated rotation
via owner+58 proxy -> proxy+10 ChrIns -> ChrIns+190 bag -> bag+68 physics+60.
Native helpers 3C6D40, 405090 and 4052C0 establish rotation of -Z, horizontal
normalization of both facing and negative incoming vector, dot/cross quadrant.
The getter env222 classifies the same pair stored at action-flag module+38.
Do not invoke that getter or write its state; normal damage would store later.

Implement finite/non-degenerate validation and pure equivalent direction math.
Compare against isolated native fixtures; preserve exact +/-45-degree table
boundaries (cos bits 3F3504F3). No position-only or invalid-vector fallback.
Gate raw input reads on verified WW 2.7.1.0 metadata and selected native
instruction fingerprints, checked once at hook initialization. Old builds
keep prior auto-dodge/Atk outputs and emit no new bearing markers.

Pack direction into unused high bits of the existing atomic pending hit so
the first contact publishes Atk key and bearing together. Store no pointers.
PreBehavior consumes once, clears all prior output markers, publishes Atk
direction and received bearing before generic 880922. Rearm/suppression unchanged.
Use neutral 0.5s CSV rows, one output per valid native enum; exit cleanup includes
all four. Unknown Atk does not invalidate a separately valid bearing.

Automatic probes: native fixture vectors/rotations/boundaries, invalid numeric
data, pending packing and CAS first-contact, output/cleanup sets, version gate.
Human probes remain exact physical sides/projectiles, absence of hit effects,
movement between capture/consume, role/buff exit and normal-skill exclusion.
Entrance snapshot intentionally precedes any later native special-case payload
rewrite. Full equivalence with every normal-damage branch is not yet claimed.
Delivery is a separate development DLL; published v0.3.2 remains unchanged.
