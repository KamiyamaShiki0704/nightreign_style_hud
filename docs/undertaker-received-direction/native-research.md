# Native Received Damage Direction Research

Date: 2026-09-08. Input: `spec.md@1.0`, UDB-01..05.
Research only: no gameplay, HKS, Param, state or plan changes; no game execution.

## Conclusion

The native direction calculation and classifier are identified and independently
exercised using copied EXE math routines. The current dispatcher supplies a
direction vector at `R8+0x1C0`; the victim's interpolated quaternion supplies the
other input. Neither attacker-position approximation nor damage execution is
needed to classify that entry snapshot. The existing Atk row/table capture alone
does not contain these inputs.

Names are now confirmed in the installed game's compiled `common_define.hks`:
**LEFT=0, RIGHT=1, FRONT=2, BACK=3**. The earlier missing-name limitation is resolved
for that explicitly hashed local support file. Entry-snapshot classification is
not a proof of equality with every later normal-pipeline getter result.

## Evidence Identity

- EXE: `F:/SteamLibrary/steamapps/common/ELDEN RING/Game/eldenring.exe`.
- SHA256: `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`.
- Size: 87,042,128 bytes. Worldwide ER 1.17.1 / internal 2.7.1.0.
- All addresses below are RVAs; preferred image base is `0x140000000`.
- HUD revision at initial investigation: `17590359edb382f1098c2cb3161d28fb7a0cbb5c`;
  parent compatibility publication now at `215a2158bf32e5027dc9167368ca68bcee705368`.
- Read-only fsrs revision: `3c8c1d7633a99309fb004c9f894ea10b7967d0e0`;
  KB revision: `9cff53bb208e0be3d39acfeffb224387224ca59a`.
- Local GA `c0000.hks` SHA256:
  `B0B78F05997C9FA1EB884B30D0E332CF684667683B1BBD096C6CF132960CD3E3`.

## Getter and Write Chain

Local `F:/GoldenAge/_GoldenAge/GA/action/script/c0000.hks:43130` defines
`GetReceivedDamageDirection = 222`. Native evidence:

| RVA | Instruction / established meaning |
| --- | --- |
| `0x410D9A` | `cmp edx,0xDE`; `0x410DA6` branches to `0x411192` on equality |
| `0x411192` | `mov rax,[rcx]`: query context's ChrIns |
| `0x411195` | `mov rcx,[rax+0x190]`: module bag |
| `0x4111A1` | `mov rcx,[rcx+8]`: action-flag module |
| `0x4111A5` | Calls `0x4106B0`, with an output buffer in RDX |
| `0x4106C9` | `movsd xmm0,[rcx+0x38]`; copies the 8-byte pair to output |
| `0x4111AE` | Calls `0x4052C0`; `movzx edi,al` retains the numeric enum |
| `0x4135AD` | Returns EDI through EAX |
| `0x445EF9` | `lea rbx,[rsi+0x1C0]`, RSI is damage payload |
| `0x445F05` | Loads victim `ChrIns+0x58` as RCX |
| `0x445F09` | Calls facing-vector helper `0x3C6D40` |
| `0x445F19` | Calls `0x405090(out8, facing16, payloadVector16)` |
| `0x446037` | Loads the calculated pair into R9 |
| `0x446045` | Calls `0x446080`, passing that pair by value in R9 |
| `0x4460A7/AE` | Resolves owner bag, then bag+8 into RBX |
| `0x44618E` | `movsd [rbx+0x38],xmm0`: writes the pair to action-flag module |

Thus action-flag `+0x38/+0x3C` stores **two floats `{dot,sign}`**, not an enum.
The getter classifies the stored pair. Rejecting the dispatcher skips this write;
reading this getter afterward can therefore return stale state.

## Entry Inputs and Convention

At dispatcher `0x449D30`, RCX is victim damage module and R8 is payload.
`0x43D250` is `mov rax,[rcx+8]; ret`, proving the damage module owner offset.
The full facing-helper access chain is:

```text
damageModule+0x08 -> owner ChrIns
owner+0x58       -> proxy
proxy+0x10       -> ChrIns
ChrIns+0x190     -> module bag
bag+0x68        -> physics module
physics+0x60    -> quaternion (x,y,z,w)
```

`0x3C6D5F..0x3C6D71` resolves that chain and calls `0x45ECA0`, whose entire
getter is `lea rax,[rcx+0x60]; ret`. fsrs `cs/chr_ins/module/physics.rs` identifies
this field as **interpolated_orientation**, not `orientation` at `+0x50`.
Do not silently substitute the latter or assume the proxy-owner link is identical
in every special topology without checking it.

`0x3C6D40` transforms **(0,0,-1)** by that quaternion. Its basis source is
`0x48064E0`, initialized to +Z by `0x105E20`, then negated at `0x3C6DF7`.
Identity quaternion yields facing `(0,0,-1)` in the isolated native test.

Payload XYZ is at **+0x1C0/+0x1C4/+0x1C8**. The native helper reads 16 bytes.
At `0x405126` the payload vector is negated using sign-bit XOR; horizontal
projection and normalization of both vectors occur in `0x4050B8..0x40518D`.
For finite nondegenerate horizontal inputs, the semantic calculation is:

```text
F = normalize_horizontal(facing)
V = normalize_horizontal(-payloadVector)
dot  = clamp(F dot V, -1, 1)
sign = sign((F cross V).y)
```

Cross product/sign: `0x405190..0x40522F`; dot, clamp helper `0x133F90`, and
pair stores: `0x40522F..0x405258`. Up is (0,1,0), initialized by `0x105DF0`
at `0x48062D0`. This is an f32 SSE calculation, not atan2 or Atk direction.
Do not negate the payload twice. Matching the semantic formula does not by itself
prove bit-identical results if scalar operation order, precision, or FMA differs.

## Exact Classification and Names

Classifier `0x4052C0..0x405303` scans three 16-byte rows at `0x3B1A018`, in order.
Each row tests inclusive dot bounds and `row.sign_multiplier * sign >= 0`.
`0x4052FC` returns 3 if no row matches. Startup initializer `0x93DF0` fills
the cosine boundaries; the raw file's zero placeholders are not runtime thresholds.
Executing that initializer gives `c = 0.7071067690849304f`, bits `0x3F3504F3`.

| Order | Dot interval | Sign multiplier | Native enum / confirmed name |
| --- | --- | --- | --- |
| 1 | `[+c,1]` | 0 | 2 / FRONT |
| 2 | `[-c,+c]` | +1 | 1 / RIGHT |
| 3 | `[-c,+c]` | -1 | 0 / LEFT |
| fallback | no matching row | n/a | 3 / BACK |

For finite valid pairs, +c belongs to FRONT, -c to a side. At side intervals,
sign=0 matches RIGHT first. These are approximately 45/135-degree divisions;
the **f32 dot boundaries and row order**, not rounded angle comparisons, are exact.
With identity quaternion, payload +Z/-Z/+X/-X gives 2/3/1/0 respectively.

Naming evidence is the installed support file
`F:/SteamLibrary/steamapps/common/ELDEN RING/Game/action/script/common_define.hks`
(175,611 bytes), SHA256
`A9B8A86B760DC4D1B33001DA161AC34302707A7B612A2FA68707FEB0FA7A3737`.
DSLuaDecompiler output `target/research-direction/common_define.decompiled.hks`
lines 434..437 explicitly assigns LEFT=0, RIGHT=1, FRONT=2, BACK=3; output SHA256
`91764DF63CF5FB788D8D8AC129AAA82743108C0474646B4FC651A12F9459AD65`.
This is direct local HKS evidence, not a transferred Sekiro constant table.
The support file's provenance has not been independently compared against the
current Steam BDT/BHD archive, nor has actual mod-loader selection been traced.

Under UDB-02, confirmed names map FRONT/BACK/LEFT/RIGHT to
880935/880936/880937/880938 respectively. **Only validated inputs may emit a marker.**
The native helper maps zero or purely vertical payload to `(0,0) -> RIGHT`;
tested NaN payload produces `(NaN,NaN) -> BACK`. These are native fallthroughs,
not evidence of a valid bearing. Missing pointers, nonfinite data, and degenerate
horizontal vectors remain unknown/no marker under Spec; never use stale getter data.

## Pipeline Limits

The identified local writer path is dispatcher `0x449D30` -> call at `0x44A169`
to `0x446CB0` -> conditional call at `0x4470E9` to `0x445EC0` -> `0x446080`.
Earlier dispatcher calls include `0x447810`, `0x448910`, and `0x448870`.
The direction writer is later than these calls and is not unconditional.

Upstream vector writes were also identified at `0x443E66` (direction reconstructed
from an input angle), `0x443E81` (source facing), `0x44AC6F`, `0x44AD76`,
`0x44B0AC` (normalized positional difference), and `0x44B507`.
These are native input preparation paths, not authorization to replace the
captured vector with a new attacker-position calculation. For example,
`0x44D148` calls `0x443A50` before `0x44D1E0` calls `0x444B40`; the latter
passes its payload into `0x449D30` at `0x444C8A`.

**Proven:** entry payload plus entry interpolated orientation is sufficient to
run the identified native mathematical classifier without running damage or
writing action-flag state. **Not proven:** every normal-path later getter equals
that entry result, all transitive calls leave inputs unchanged, or every special,
projectile, network, mounted, and invalid-data case is covered. No live collision,
HP, audio, animation, latching, or SpEffect acceptance test was performed here.
Restrict any implementation to the audited WW version; old versions/regions are
not covered. Human acceptance and parent Build remain separate from this research.

## Reproduction

Preserved ignored artifacts live in `target/research-direction/`:
`probe.py` verifies the EXE hash and prints bytes/instructions/RIP targets;
`native_math.py` copies the PE into its own process and invokes only explicitly
allowed math/constant-initialization helpers, never the game entrypoint/imports
or any damage routine. `native-math-results.json` records actual table bits,
cardinal/diagonal/invalid samples, boundary-adjacent floats and quaternion samples.
Its NaN samples use Python JSON's nonstandard NaN literal; do not treat them as
strict-JSON fixtures without conversion. Some PE unwind entries are function
fragments; raw xref scans are candidates, not exhaustive control-flow proofs.

```powershell
D:/Python/python.exe target/research-direction/probe.py dump 411192 4111C5
D:/Python/python.exe target/research-direction/probe.py dump 4052C0 405304
D:/Python/python.exe target/research-direction/probe.py func 445EC0
D:/Python/python.exe target/research-direction/native_math.py
& 'D:/_Ugit/FSTool/FSModify/UnpackingTools/DSLuaDecompiler/net9.0/DSLuaDecompiler.exe' 'F:/SteamLibrary/steamapps/common/ELDEN RING/Game/action/script/common_define.hks' --output 'D:/_Ugit/FSTool/_Mod/_Project/nightreign_style_hud/target/research-direction/common_define.decompiled.hks'
```

Decompiler EXE SHA256: `32B1A08C82BB2E353CCBA9AB22824334A5FA83135FB7915AFE41694362AA3A02`;
LuaDecompilerCore.dll SHA256: `21E9A1E147863782F43464BA14E4D896C691A14F4A91ED173A2CD2E5DBA64654`.
One unrelated address correction observed: dispatcher call `0x449DFD` targets
Atk resolver **0xD247D0**, not the old `0xD24760` in the gameplay source comment.
