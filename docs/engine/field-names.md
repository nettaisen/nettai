# Field names: the `unk_*` mapping

The engine no longer has fields named after an offset or address of the game's structures (`unk_2c`, `unk_2036720`).
Each such field was either given a name for its role or deleted. A field was deleted when no ported code reads it.
Some of those were written but never read, or only counted down. For deleted fields, the routines that use the offset
in the game are listed below, for whoever ports them later.

Evidence is cited by routine name (`sub_801A36A`) and struct offset (`AIData+0x38`). "No reader found" means a scan of
the disassembly found no symbolic reference and no raw access through a register holding the struct pointer. That
scan is heuristic, not a proof.

Behaviour is unchanged: the golden-trace replays match the same number of frames before and after the pass
(machgun 1074/552, soundmod 2933/3462/1947, with main's movement and chip-use work merged in).

For fields likely to come back with a later port, the evidence column suggests a name ("re-add as ...").

## Fixing up code that uses the old names

Most renames are mechanical. These changed type, so their uses change too:

| Old use | New use |
|---|---|
| `ai.unk_32 != 0` / `ai.unk_32 = 0xFFFF` | `ai.beast_out_spent` / `ai.beast_out_spent = true` |
| `ai.unk_36 != 0` | `ai.beast_over_exhausted` (writing 0x3C0 becomes `= true`) |
| `ai.unk_1c != 0` / `= 0` / `= 1` | `ai.hit_bug_latched` / `= false` / `= true` |
| `ai.unk_3c` (u16) | `ai.bubble_base_z` (i16; `(z as i32) << 16` gives the same bits) |
| `ai.unk_40` (already `Option<ObjectRef>`) | `ai.lockon_marker` |
| `ai.unk_50 != 0` (u32) | `ai.reset_linked_object.is_some()` (`Option<ObjectRef>`) |
| `ai.unk_5c == 0` / `= 0` (u32) | `ai.full_synchro_aura.is_none()` / `= None` (`Option<ObjectRef>`) |
| `obj.unk_0d` (0 / 4 / 8) | `obj.drag_step` (`DragStep::Start` / `Slide` / `Recover`; `object::DragStep`) |
| `obj.unk_30`, `obj.unk_32` (u16) | `obj.shake_origin_x`, `obj.shake_origin_z` (i16; `(pos >> 16) as i16` to store) |

Deleted fields have no replacement. If a branch reads one, that code is new: give the field a name for its role
(see the evidence below; several suggest one), then add it back. A Beast Out port will likely want
`AttackVars::unk_1e` back, as `beast_step`.

`AttackVars::move_arg` (not an `unk_*` name) was also deleted: after the merge nothing writes or reads it.

## ActorData (`actor.rs`; the game's AIData, 0x100 bytes per slot)

Fields that already had names are unchanged. `drain_counter` (+0x0A) kept its name; only its doc comment changed.

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_03` | deleted | u8 | AIData+0x03: byte 2 of the actor's enemy record (`enemy_getStruct1`), copied at spawn (`sub_800753C`); 1 for player navis. Read by `sub_800F334` and `sub_81095D0` (virus code). The port wrote it at spawn but never read it. |
| `unk_09` | `hp_drain_counter` | u8 | AIData+0x09: ticks toward the next HP lost to the fight-time HP bug. Players: `sub_8010230` (level from NaviStats+0x18). Actors without navi stats: `sub_801026A` (level from AIData+0x12). |
| `unk_0b` | deleted | u8 | AIData+0x0B: the emotion last picked by the NaviCust emotion-swing bug (`sub_8013DA0`, unported). Re-add as `swing_emotion`. |
| `unk_0c` | deleted | u8 | AIData+0x0C: written by virus/navi init `sub_8016F56` (the opponent's max base HP / 100, clamped to 1..10). Read by `sub_800FE12` and `sub_800FE36`, which multiply a table value by it for actors of version 4. |
| `unk_0d` | deleted | u8 | AIData+0x0D: count of absorbed obstacles (`sub_80E991C`, `sub_8011F8C`, `sub_8011FCE`). Already modeled as `absorbed.len()`. |
| `unk_0e` | deleted | u8 | AIData+0x0E: set to 0xFF at spawn (`sub_800753C`) and at deletion (`sub_8016C4E`). Its only reader, `sub_800A86E`, has no effect. The port wrote it at spawn and in `destroy`; both writes are gone. |
| `unk_0f` | `beast_out_check_delay` | u8 | AIData+0x0F. The turn-start Beast Out check (`sub_80159C6`) runs only while this is 0, then sets it to 2. Closing the custom screen sets it to 1 (`sub_8009338`). A mid-battle custom-screen request counts it down (`sub_8015A16`: MegaMan only, not below 0, 0xFF untouched). **Intent uncertain:** in netbattles the close always leaves 1 before the next check. |
| `unk_10` | `drain_heal_credits` | u8 | AIData+0x10: drain hits this navi landed on the opponent (`sub_801A308`/`sub_801A324`: the opponent's CollisionData+0x92 is added here). On its own next hit collection it heals MaxHP/10 per credit and clears the count (`sub_801A324`). |
| `unk_12` | deleted | u8 | AIData+0x12: HP-bug level of actors without navi stats. Read by `sub_801026A`; raised by bug code 0x18 (`sub_8013B20`) and bug code 0xF6 (`sub_801A75A`). Re-add as `hp_bug_level`. |
| `unk_13` | `back_special_window` | u8 | AIData+0x13: ticks left to press Back after B for the B+Back special. `sub_8012FC8` sets 8 on a B press and counts it down. Also read by `sub_8112B06`. |
| `unk_14` | deleted | u8 | AIData+0x14: padding in the game's struct. No access found. |
| `unk_15` | `back_special_cooldown` | u8 | AIData+0x15: ticks before B+Back can be input again. Set from AIAttackVars+0x05 (the lockout) when a kind-3 action ends (`sub_801171C`). Kind 3 is `object_setAttack3`, which only the B+Back request uses (`sub_80F0354`). Counted down by `sub_80107D4`; `sub_8012FC8` skips B+Back while it is nonzero. |
| `unk_16` | deleted | u8 | AIData+0x16 (`Version_16`): the actor record's version byte, copied at spawn (`sub_800753C`). Read by about 80 virus/navi AI routines. The port wrote it at spawn but never read it. `NaviRecord::version` still carries the value. |
| `unk_17` | deleted | u8 | AIData+0x17 (`Version_17`): the same copy. Read and written by `sub_81095D0` only. |
| `unk_18` | deleted | u8 | AIData+0x18: counted down by `sub_802DD62` (Cross code); no other access found. |
| `unk_1c` | `hit_bug_latched` | bool | AIData+0x1C: the NaviCust on-hit bug (NaviStats+0x16) already fired during this hit sequence. `sub_8013F1E` clears it when it runs with `prevent_anim` 0. The game stores 0/1. |
| `unk_1f` | deleted | u8 | AIData+0x1F: no reader found. |
| `unk_32` | `beast_out_spent` | bool | AIData+0x32 (the game stores 0xFFFF or 0). Set by `sub_801443C`, which is called: at init with a zero Beast Out counter (`sub_8013892`); by the turn-start check (`sub_80159C6`); when a Beast Out (not Over) reverts (`sub_80158CC`); by the emotion-swing bug (`sub_8013DA0`); and by `sub_80E4954` when the counter is 0. Cleared by `sub_8014446` (from the emotion-swing bug, and from `sub_80E4954` when the counter is not 0). Effects: emotion 1 (`sub_8015B64`), mood changes blocked (`sub_8015BEC`), anger blocked (`sub_80143CE`), counter-hit Full Synchro blocked (`sub_801A200`). |
| `unk_36` | `beast_over_exhausted` | bool | AIData+0x36: set when a Beast Over form reverts (`sub_80158CC` → `sub_8014466`, which also sets mood 0). The game stores 0x3C0, but nothing counts it down, so it is a flag. Effects: emotion 5 (`sub_8015B64`), mood changes and anger blocked, and 1 HP lost per tick, never the last one (`sub_8014498`). |
| `unk_38` | `road_cooldown` | u16 | AIData+0x38: ticks before a road panel can start another slide. Set to 5 after a road slide (`sub_80166D0`, `sub_8016730`) and to 1 by `sub_80F650A`. Counted down and tested by `sub_801A36A`; tested by `sub_801A400`. |
| `unk_3a` | deleted | u16 | AIData+0x3A: the emotion-swing bug's 60-tick counter (`sub_8013DA0`). Re-add as `swing_timer`. |
| `unk_3c` | `bubble_base_z` | i16 (was u16) | AIData+0x3C: the height (Z16, whole pixels) a bubble bobs around and restores when it pops (`sub_8016B72`, `sub_801A2B0`). Viruses record it every tick when not bubbled (`sub_8108F74`); nothing sets it for players. |
| `unk_3e` | deleted | u16 | AIData+0x3E: no reader found. |
| `unk_40` | `lockon_marker` | `Option<ObjectRef>` | AIData+0x40: the Beast Out lock-on marker (effect #0xF, spawned by `sub_80E1620`). `sub_80E164A` reads its panel; `sub_80E1654`/`sub_80E1662` freeze and unfreeze it; `sub_801562C` clears the pointer. |
| `unk_4c` | `stun_ticks` | u32 | AIData+0x4C: consecutive ticks spent flinching or paralyzed (`sub_80143FC` via `sub_8014432`/`sub_8014424`). At 120, base MegaMan gets angry (`sub_80142DC`). Cleared when anger ends (`sub_80143A6`/`sub_80143B4`). |
| `unk_50` | `reset_linked_object` | `Option<ObjectRef>` (was u32) | AIData+0x50: an object tied to the navi. The full status reset ends it (`sub_80144C0` → `sub_801390C` → `sub_80E5410`: state 8, first extra var cleared) and clears the pointer. **Uncertain:** no routine that stores an object here was found. |
| `unk_54` | deleted | u32 | AIData+0x54: read by `sub_801B9BC`. |
| `unk_5c` | `full_synchro_aura` | `Option<ObjectRef>` (was u32) | AIData+0x5C: the Full Synchro aura (actor #0x5E). Spawned while the emotion is 2 and no aura exists (`sub_80139C4` → `sub_80C4C12`). Form changes end it through `sub_80C4C3A` (`sub_8014B18`, `sub_8014D08`, `sub_8014F40`, `sub_801516C`, `sub_80153EC`). Visibility follows the navi (`sub_80E1352`, `sub_80E13DC`). Cleared at deletion (`sub_801746E`) and by `sub_802D950`. |
| `unk_60` | deleted | u32 | AIData+0x60: the barrier visual object. Cleared with the barrier by `sub_801A7F4`; read by the barrier code (`sub_80DE088`, `sub_80E3AFC`, `sub_810AA90`) and by visibility helpers (`sub_80E1352`, `sub_80E13DC`, `sub_80E146C`, `sub_80E14AC`). The port cleared it at deletion but never read it. Re-add as `barrier_visual: Option<ObjectRef>`. |
| `unk_64` | deleted | u32 | AIData+0x64: no reader found. |
| `unk_6c` | deleted | u32 | AIData+0x6C..+0x6F: the first four absorbed-obstacle entries (`sub_80E991C`). Already modeled by `absorbed`. |
| `unk_70` | deleted | u32 | AIData+0x70..+0x73: the last four absorbed-obstacle entries. Already modeled by `absorbed`. |
| `unk_74` | deleted | u32 | AIData+0x74: an object handed to `sub_80E1A86` by the virus/navi deletion routines (`sub_8016F1A`, `sub_8017122`, `sub_80171D8`); cleared by `sub_801664E`. Also read by `sub_80BC36E`, `sub_80BE6BC`, `sub_80C0072`, `sub_80C10AC`, `sub_80C13E4`. |
| `unk_78` | deleted | u32 | AIData+0x78: the actor's target, the opposing player navi, stored by `sub_800F318` from virus/navi init `sub_8016F56`. Read by `sub_800F2F0`, `sub_800D4AC` and navi AI (`sub_8101AD4` ...). Re-add as `target: Option<ObjectRef>`. |

## AttackVars (`actor.rs`; AIData+0xA0, the game's AIAttackVars)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_16` | deleted | u8 | AIAttackVars+0x16: for the player's step (`sub_80EB088`), the destination panel's x (the absolute step's target on entry, the chosen target after). The ported step keeps it as `movement::Vars::target`. Navi/virus AI and other chip routines use the byte as their own scratch (about 80 routines, e.g. `sub_80ECF8E`, `sub_80F162C`). |
| `unk_17` | deleted | u8 | AIAttackVars+0x17: the step's destination y, as above. |
| `unk_18` | deleted | u16 | AIAttackVars+0x18: a step's end lag in ticks, set when a step starts (`sub_80116AE`, `sub_80116D8`); the step copies it into its timer (+0x10) on arrival (`sub_80EB194`). The ported step keeps it as `movement::Vars::end_lag`. (Before the merge this pass had renamed it `move_lag`.) Navi AI uses the halfword too (about 120 routines). |
| `unk_1a` | deleted | u8 | AIAttackVars+0x1A: navi AI scratch. Written by about 76 AI routines; read by `sub_80F59E8`, `sub_8101E24`...`sub_8101EE2`, `sub_810A080`, `sub_811239A`. |
| `unk_1e` | deleted | u16 | AIAttackVars+0x1E: the Beast Out attack wrapper's step. `sub_80EAD9C` dispatches on the byte at +0x1E; `sub_801011A` (`reset_attack_links`, from every `set_attack`) clears the halfword. The wrapper is not ported, so the port's clear is gone. Re-add as `beast_step: u8` (plus the byte at +0x1F if the wrapper needs it). |
| `move_arg` | deleted | u32 | AIAttackVars+0x2C: the absolute step's panel-trail argument (`sub_80116AE`/`sub_80116D8` store 0). After the merge nothing writes or reads it. |

## Object (`object/mod.rs`; the game's BattleObject)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_0c` | deleted | u8 | BattleObject+0x0C: per-kind scratch. For sprite attachments (actor #5) it is a signed pixel lift subtracted from Y and Z; the ported attachment keeps it as `attachment::Vars::lift`. The Full Synchro aura's spawner sets it to 1 (`sub_80C4C12`). Many attack and effect kinds use it their own way (`sub_8017E44`, `sub_80D65FC`, `sub_80BD084`, `sub_80C0C48`...). |
| `unk_0d` | `drag_step` | `DragStep` (was u8) | BattleObject+0x0D: the drag reaction's step: 0 start (`sub_80178D4`), 4 slide (`sub_8017992`), 8 recover (`sub_8017A38`). Dispatched by the drag actions of every actor kind (players `sub_80178B6`; others `sub_8016CE8`, `sub_8017CC0`, `sub_8017E26`). Zeroed by stage B on every undragged tick (`sub_801AF44` and its per-kind twins `sub_801B1C4`...`sub_801B878`). Attack objects use the byte for other things (`sub_80C0DD8`, `sub_80EA11C`, `sub_80DA37A`). |
| `unk_19` | `shake_timer` | u8 | BattleObject+0x19: ticks left of the time-stop shake. `sub_8017AB4` sets 30 per damaging hit and zeroes it on entry. Other kinds use the byte for other things. |
| `unk_30` | `shake_origin_x` | i16 (was u16) | BattleObject+0x30: the whole-pixel X an actor shakes around in time stop, saved from X16 on the handler's first tick (`sub_8017AB4`). Other kinds use the halfword for other things (e.g. a time-freeze chip's id). |
| `unk_32` | `shake_origin_z` | i16 (was u16) | BattleObject+0x32: the same for Z16. Other kinds use it for other things (e.g. a time-freeze chip's bonus). |

New type: `object::DragStep { Start, Slide, Recover }` (the game's 0, 4, 8). `Default` is `Start`.

## CollisionData and Accumulators (`collision.rs`)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `CollisionData::unk_54` | deleted | u32 | CollisionData+0x54: `object_presentCollisionData` stores the caller's r1 there, then 0 unless in time stop. No reader found. The port's clear in `present_collision` is gone. |
| `CollisionData::unk_5c` | deleted | u32 | CollisionData+0x5C: no access found. |
| `CollisionData::unk_64` | deleted | u32 | CollisionData+0x64: a per-variant value set by one attack kind (`sub_80C518C`). `sub_3007218` adds it into the receiver's +0xA0. The port never set it, so it was always 0. |
| `Accumulators::unk_7a` | deleted | u16 | CollisionData+0x7A: the upper half of the game's u32 at +0x78. Only the lower half (`elec_damage`) is accessed. |
| `Accumulators::unk_a0` | deleted | u32 | CollisionData+0xA0: sum of the hitters' +0x64 (`sub_3007218`). No reader found. The port's accumulation line in `resolve_hit` is gone. |
| `Accumulators::unk_a6` | deleted | u16 | CollisionData+0xA6: the upper half of the game's u32 at +0xA4. `inflicted_bugs` is written with 16-bit stores, and the present clears the rest. No separate access found. |

## Round and battle state (`battle.rs`)

`RoundState` (the game's BattleState) had no `unk_*` fields. The ones in this file were on the structures below.

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `FightMachine::unk_1` | deleted | u8 | Byte 1 of the fighting-phase machine (the 0xC-byte block at 0x0203CA70). No ported reader. |
| `SideState::unk_02` | deleted | u8 | `sub_802E070(side)`+0x02: set to 0xB4 by `sub_802E07C`. Battle flag 0x40 mode only. |
| `SideState::unk_03` | deleted | u8 | +0x03: set to 0 by `sub_802E07C`. |
| `SideState::unk_0b` | deleted | u8 | +0x0B: set to 0xFF by `sub_802DFC8`. |
| `SideState::unk_0e` | deleted | u8 | +0x0E: set to 3 by `sub_802DFC8`. |
| `SideState::unk_10` | deleted | u8 | +0x10: set to 1 by `sub_802DFC8`. (`panel_x` is +0x11.) |
| `SideState::unk_18` | deleted | [u32; 3] | +0x18..+0x23: set to 0xFFFFFFFF by `sub_802E07C`. |
| `SideState::unk_2a` | deleted | u16 | +0x2A: set to 0 by `sub_802E07C`. (`gauge` is +0x28.) |
| `SideState::unk_2e` | deleted | u16 | +0x2E: counted down by `sub_80107D4` while +0x00 (`active`) is set. The port's countdown in `tick_cooldowns` is gone. |
| `SideState::unk_30` | deleted | u16 | +0x30: counted down in the player's stage B (`sub_802E1D8`); tested by MegaMan's idle controller in the flag-0x40 branch (`sub_80F0354`). The port's countdown in stage B is gone. |
| `SideState::unk_3a` | deleted | u16 | +0x3A: counted down by `sub_80107D4`, like +0x2E. |
| `SideState::unk_3c` | deleted | u16 | +0x3C: counted down by `sub_80107D4`, like +0x2E. |
| `LinkedRecord::unk_02` | deleted | u16 | Registry record +0x02 (0x10 bytes per side at 0x02036720): a value the registering chip passes to `sub_802CE8A` (r5). Returned by `sub_802CE60`/`sub_802CE78`; cleared by `sub_802CEA6`. Nothing ported registers a record. |
| `LinkedRecord::unk_04` | deleted | u32 | Record +0x04: a value from the registering chip (`sub_802CE8A`, r6). Returned by `sub_802CE60`/`sub_802CE78`. `sub_802CEA6` clears only its low half. |
| `LinkedRecord::unk_08` | deleted | u32 | Record +0x08: the owner object (`sub_802CE8A`, r0). `sub_802CEC8` clears the record once the owner's HP is 0. The port's `update_linked_registry` is an empty stub; that is correct only while nothing registers. |

`Battle::linked`'s doc comment named the registry by its RAM symbol. It now gives the address.

## ChipHand (`hand.rs`; the game's 0x50-byte chip block)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_01` | deleted | u8 | Chip block +0x01: always 0 in battle. Only the unreferenced flag-0x40-mode routines `sub_802DE74` and `sub_802E588` use it. `from_bytes` skips it and `to_bytes` writes 0. Every chip block in both traces has 0 there. |

## BattleSettings (`setup.rs`; the game's 16-byte record)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_01` | deleted | u8 | Settings+0x01: read through `GetBattleSettingsUnk01` by `sub_8026F1A` and `sub_80AA4C0`, outside the battle simulation. It is 0x64 in both traces. `netbattle_from_bytes` skips it. |
| `unk_07` | deleted | u8 | Settings+0x07: no reader found; 0 in both traces. |

## ChipData (`data/mod.rs`, emitted by `bn6-extract`)

| Old | New | Type | Meaning and evidence |
|---|---|---|---|
| `unk_0d` | deleted | u8 | Chip record +0x0D: no reader found (docs/engine/chips.md §1.2). No longer extracted. |
| `unk_0e` | deleted | u8 | Chip record +0x0E: values 0/4/5/6; no reader found. No longer extracted. |

`chips_generated.rs` was regenerated. No other generated table changed.

## Constants, enum variants and functions

These are not fields, but they were `UNK_*` names or were named after the `UNK_4` bit.

| Old | New | Meaning and evidence |
|---|---|---|
| `battle::battle_flags::UNK_20` | deleted | Battle flag 0x20: no setter found. Cleared by form-change code (`sub_8014CC0`, `sub_8014F04`, `sub_8015128`); read by `battle_isTimeStopPauseOrBattleFlags0x20_800a0a4`. Unused by the port. |
| `collision::f1::UNK_4` | `collision::f1::SEMI_INTANGIBLE` | ObjectFlags1 0x4. The object collides only with collision types that have bit 0x8 or 0x1000, in both directions (`sub_3007218`). `sub_8010162` holds it each tick while the timer below runs and no action is in use. **Uncertain** which chip or state uses it: `sub_80101AE` starts it for 480 ticks and hides the navi, from a navi-double object (`sub_80C49E4`); a navi AI sets the bit directly (`sub_80FDEFC`). |
| `collision::timer::UNK_26` | `collision::timer::SEMI_INTANGIBLE` | CollisionData+0x26: the timer for the state above (0xFFFF = indefinite). Started by `sub_80101AE`; ended by any hit (`sub_8010198`) or by `sub_80101C4`. |
| `data::player::StatusTimer::Flag4` | `StatusTimer::SemiIntangible` | A status-table entry that writes CollisionData+0x26. None of the extracted entries does. Renamed in the extractor too. |
| `kinds::obstacle::f2::UNK_4` | `kinds::obstacle::f2::FLINCH` | ObjectFlags2 0x4: a flinch request (hit modifier bit 0x1, as for players in `sub_801AEB0`). A push cancels it. |
| `kinds::player::cancel_flag4_timer` (private) | `cancel_semi_intangible` | `sub_80101C4`. |
| `kinds::player::status::tick_flag4_timer` (private) | `tick_semi_intangible` | `sub_8010162`. |
| `kinds::player::intake::hit_cancels_flag4_timer` (private) | `hit_ends_semi_intangible` | `sub_8010198`. |

## Names left as they are

These are named after a bit value or an object index, not a struct offset or address, so this pass left them alone.
They are candidates for a later naming pass: `request::PAUSE_40`, `request::PAUSE_4000000`, `request::TRAP_200`,
`request::TRAP_400`, `request::TRAP_8000`, `request::ACTION_30`, `request::ACTION_49`, `status::CROSS_2000` ...
`CROSS_40000`, `battle_flags::MODE_40` (and `is_mode_40`), and `setup::ActorKind::Object6E` / `Object7D`.
