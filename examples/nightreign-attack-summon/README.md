# nightreign-attack-summon

Standalone DLL for parameter-side Speffect triggers that spawn temporary attack entities around the local player.

## Speffects

- `14200120`: spawn NPCParam `42050100`, NPCThinkParam `42050000`, animation `20010`.
- `14200121`: spawn NPCParam `42050101`, NPCThinkParam `42050000`, animation `20011`.
- `14200122`: spawn NPCParam `42050102`, NPCThinkParam `42050000`, animation `20012`.
- `14200123`: spawn NPCParam `42050103`, NPCThinkParam `42050000`, animation `20013`.
- `14200124`: spawn NPCParam `42050104`, NPCThinkParam `42050000`, animation `20017`.
- `14200125`: spawn NPCParam `42050105`, NPCThinkParam `42050000`, animation `20018`.
- `14200128`: marker applied by the DLL to generated units.
- `14200129`: vanish/end request. Add this to the generated unit near the animation end to make the DLL remove it.

Generated units use model `c4205`, CharaInit `0`, and are assigned team/type `47` after spawn.

## Behavior

The DLL consumes trigger Speffects from the player as one-shot requests. Each spawned unit is placed in front of the player using the current camera direction, plays its configured animation, and is removed when it receives `14200129`, dies, disappears, or reaches the fallback timeout.

## Configuration

Place `summon.toml` next to the DLL:

```text
F:\GoldenAge\_GoldenAge\dll\Summon\summon.dll
F:\GoldenAge\_GoldenAge\dll\Summon\summon.toml
```

The DLL reads this file once at startup. If the file is missing or invalid, the built-in defaults are used. Restart the game after editing the file.

Configurable global fields:

- `marker_speffect`
- `vanish_request_speffect`
- `generated_team_type`
- `max_active_summons`
- `unbound_timeout_ms`
- `fallback_lifetime_ms`
- `spawn_forward_distance`
- `spawn_height_offset`
- `same_trigger_cooldown_ms`
- `any_trigger_cooldown_ms`
- `reuse_spawned_units`

When `reuse_spawned_units = true`, each summon entry keeps one generated entity in an idle hidden state after it ends. Later triggers reuse that entity instead of creating another debug character. This keeps the debug/enemy pool from growing every time the effect is used.

Each `[[summons]]` entry can customize:

- `trigger_speffect`
- `chr_id`
- `chara_init_param_id`
- `npc_param_id`
- `npc_think_param_id`
- `event_entity_id`
- `talk_id`
- `animation_id`
- `side_offset`
- `is_player`

Build:

```powershell
cargo build --package nightreign-attack-summon --release --target x86_64-pc-windows-msvc
```
