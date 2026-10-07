# Rust and Luau: where the line runs

The line between the engine's Rust and a game's Luau, as the user approved it on 2026-10-05 ("okay do that"). It
replaces nothing in [rules-in-luau.md](rules-in-luau.md) §2.1 (the layers, and which rules stay Rust for speed); it
says what each side may know. Every change from here on moves code toward it, and the crossings below are the ones
still to remove.

## The line

- **Rust is the machine, Luau is the game.** Rust runs objects, collision and hits, the panel grid, the custom
  screen's chip window, the flow, rollback and the digest, input, drawing and sound. What a game is (its forms, its
  custom-screen extras, its save, its NaviCust, its patch cards, its auto battle) is its Luau.
- **Rust never names a game or a game's feature**, only roles and schemas. It knows a fact by its role
  (`PlayerFact::Version`, `Level`), a definition by its registry and handle, a state or setup by the schema its
  content declares. It knows no "beast", no "souls", no "exe5".
- **Luau does no I/O and reads no clock.** Content gets what it reads from the engine's API: the battle's state, its
  own state and setup, its definitions. Nothing it runs can tell two machines apart, so both peers of a netbattle
  and every replay compute the same.
- **Rust calls Luau at defined points**: a hook at an event (a turn's start, a chip's use, a custom-screen request),
  and a state's tick while a state the rules set is active (a controller, a wrapper, a window). Never once per drawn
  frame: what the frontend draws, it reads from typed views of the state (`Battle::form_list`, `Battle::offer`).
- **Tools are generic over the schema.** A match file, the editor, a netplay offer and a match's description are
  made from the rules' setup schema: a game that declares another fact has it in all of them without a line of Rust.

## How a game's rules reach Rust

A game has one rules definition (its root's `rules`, rules/init.luau; [rules-in-luau.md](rules-in-luau.md) §2.5),
written by hand as a whole: a side's state and a player's setup stated in one place, each hook a plain function that
calls the game's modules (the save, Beast Out, Soul Unison, the NaviCust, ...: ordinary modules of functions) in the
order its code says. The engine calls a hook by its name and reads its answer, keeps one state block and one setup
block per side, and reads a setup field it needs by its role. Which module answers, and in what order they run, is
the rules' code.

## Known crossings

Every place where Rust still knows one game's feature, from an audit of nettai-battle, nettai-render,
nettai-frontend, nettai-match, nettai-content, nettai-assets and nettai-content-api on 2026-10-06 (engine
d55c7b5d). Not audited: the compat crates, which are the boundary and name a game by design; tests; comments.
None of these is to grow. Each entry has a kind:

- **(a) a name**: a generic mechanism under a feature's name; renaming it to a role moves nothing.
- **(b) game logic**: belongs in the game's Luau. The entry says roughly what moving it takes and what it costs per
  tick.
- **(c) presentation**: the frontend draws one game's window or picture. [rules-in-luau.md](rules-in-luau.md) §4.8
  keeps the drawing in Rust, reading views by role; making the drawing data is out of scope. The plan is the names.
- **(d) stays Rust**: framework that runs per tick or per object, under §2.1's "stays Rust for speed", and that both
  games share. At most a name changes.

What the framework shares with both games (the mood, the form break, the navi switch, the NaviCust's bugs, the dark
chip's hover) is no crossing in itself; only its names are. Line numbers are as audited.

| Feature | (a) | (b) | (c) | (d) | All |
|---|---|---|---|---|---|
| Beast Out and Beast Over | | 1 | 1 | | 2 |
| Crosses and the form framework | | | 2 | | 2 |
| Emotions | | 1 | 1 | 3 | 5 |
| NaviCust | | | | 2 | 2 |
| Souls and Chaos Unison | 1 | 1 | 1 | | 3 |
| The stat block and versions | | | 1 | | 1 |
| Tools | | | 1 | | 1 |
| **All** | **1** | **3** | **7** | **5** | **16** |

Of the audit's 48, step 1 (2026-10-06) did every name (kind (a)) but S2's part that waits on S1, and B1 and B4's
dead code; the library agent's steps did N5, T2 and T4, and the move of the importers into compat T1; step 2 does
the kind (b) entries one at a time. They are listed under [Done](#done) with their new names. X5 is new: what X4's
rename left of EXE5's logic.

### Beast Out and Beast Over (EXE6)

- **B2. The lock-on** (`ho_8026554`). Pieces:
  - actions/lockon.rs (131 lines);
  - the `lockon` section (`Lockon`, rules.rs:1065; sections.rs:616);
  - the `lockon` records (`LockonDef`, defs.rs:2113; `Content::lockon`);
  - the actor's `rush_lockon` (actor.rs:199; `ActorField::RushLockon`);
  - `CoreApi::lockon_panel`.

  Only content asks for it: EXE6's rush, the Beast claw, the lunge and GroundCross's drill. *(b)* An EXE6
  rules/lockon module: the records as plain data, the routine in Luau, `rush_lockon` a field of the rush's state.
  About 190 lines of Rust. It runs once per rush step or attack setup, never per tick.
- **B6. The HUD's Beast Out count** (`beast_count_shown`, render hud.rs:926, which reads `PlayerFact::BeastOut`) and
  that fact role (views.rs:124), which nothing else reads. (The count itself is read by its role since B3:
  `StatRole::WindowCount`.) *(c)* A HUD view the rules fill (whether the count shows). The fact stays EXE6's setup
  field, with no role.
### Crosses and the form framework (EXE6; the form break and the navi switch are both games')

- **C7. The renderer's Cross window**. Pieces:
  - render custom.rs's `cross_stage`, `cross_map`, `cross_names`, `cross_cursor_parts`, `navi_crosses`,
    `cross_picture` and `cross_at` (custom.rs:553–735, :865–890, :1354), the last reading `PlayerFact::CrossList`;
  - `lookups::cross_name` (lookups.rs:403);
  - `Lookup::{CrossName, CrossDescription}` (audit.rs:79);
  - chatbox.rs:90.

  About 250 lines. *(c)* It draws the `FormList` view (§4.8). Rename to the view, and read the offered forms from
  the view rather than from the fact.
- **C8. The Cross window's assets**. Pieces:
  - nettai-assets' `CustomScreen::{cross_maps, cross_patches, cross_cursor, cross_cursor_palette}`,
    `VersionPictures::{cross_names, cross_palettes}` and `CustomLayout::cross_names`;
  - nettai-content's custom.rs (:60, :89, :131, :265–290, :443–525, :615–686).

  About 120 lines. *(c)* The pack format names these pictures for the form list window. Rename with C7 at a pack
  format bump.
### Emotions (both games; EXE6's tired, Full Synchro and anger, EXE5's worried and dark)

- **E1. The `Emotion` enum and its derivation** (`Emotion`, `emotion()`, player/mod.rs:360–412: `sub_8015B54` read
  by the `emotion` section's `EmotionRules`; `CoreApi::emotion`, api.rs:1272; `CustomPlayer::emotion`, api.rs:876).
  The enum holds both games' emotions. Read by:
  - the palette (per tick);
  - the doubling (per chip use);
  - the HUD's faces, the custom screen's `PlayerView`, the last stand and idle.

  *(b)* By §4.6, each game's rules name their emotions and push the current one at their own events: a mood change,
  anger, exhaustion, tired. The framework keeps typed fields for what it reads on hot paths (Full Synchro, angry,
  worn out). About 50 lines plus some 15 readers. Pushing means no per-tick Luau. This is the widest-reaching item.
- **E2. The mood and anger machinery**:
  - `set_mood`, `gain_mood`, `lose_mood`, `mood_is_held` (player/mod.rs:525–565);
  - anger's tick (`tick_anger`, status.rs:930, every tick for every navi);
  - `anger_trigger` (intake.rs:857) and `counter_and_mood` (status.rs:380);
  - the hit's `mood_damage` (collision.rs:97, :554);
  - `never_angers`.

  *(d)* Both games' mood framework, which §4.6 keeps. It runs per tick and on the hit path.
- **E3. Full Synchro**:
  - the doubling and its spending (chip_use.rs:237, :597);
  - the aura kind `engine/full-synchro-aura` (full_synchro_aura.rs, 171 lines; the `effects.full_synchro_aura`
    rule; `SpriteRole::FullSynchroAura`; `ActorField::FullSynchroAura`; hidden by dimming.rs:462–497);
  - the mood's palette (player/mod.rs:1329–1346; form_overlay.rs:112).

  *(d)* The series' framework (§4.6: "Full Synchro and the mood, which the lineage shares"). The aura runs per tick.
  The name is the series' own.
- **E4. The emotion window's flicker** (`Console::emotion_window`, `update_emotion_window`, console.rs:93–320; the
  `emotion_window_glitch` API, api.rs:1286, :2038; `HudPart::EmotionWindow`). It checks every tick on both consoles
  and draws the console's RNG, which the digest covers. *(d)* It stays simulated. At most the HUD part gets a neutral
  name.
- **E5. Faces by emotion**:
  - `FaceSet` (navis.rs:450–495: normal, angry, tired, full_synchro, worn_out, worried);
  - the HUD's pick and mood flash (render hud.rs:94–104, :210–230, :846–868);
  - `face_hub` and `face_chaos` (player/mod.rs:499–520);
  - render audit's `emotion_number`.

  *(c)* Faces keyed by the game's emotion names once E1 is done. The second set of faces is a look the rules push
  (`face_hub` already is; `face_chaos` follows S1).

### NaviCust (both games)

- **N1. The bugs' effects** (`NaviCustBugs`, setup.rs:92). Where they act:
  - the HP drain (intake.rs:405; the `hp_bug_periods` rule);
  - the custom screen's drain and damage (battle.rs:2193, :1361);
  - the starting damage and the battle-start bug (player/mod.rs:1264, :1198);
  - astray steps (idle.rs:566);
  - the panel trail and auto step (movement.rs:332, :401);
  - the buster's blanks and charged shots;
  - the hand's shrink (screen.rs:1601);
  - the bug count the emotion window reads (console.rs:185).

  *(d)* Per tick or per step on the navi's hot paths, and EXE5's NaviCust has bugs too. Rename to afflictions the
  rules set; they already write them through `NaviStat`.
- **N3. The abilities in the stat block** (`float_shoes`, `air_shoes`, `undershirt`, `super_armor`,
  `status_guard`, setup.rs:159–162; `apply_ability_flags`, `init_round_state`, player/mod.rs:1008–1090). *(d)* Navi
  mechanics the framework reads per tick: panels, holes, flinching, deletion. Chips and forms give them too. (Their
  functions' `navicust` names went in step 1: N6.)
### Souls and Chaos Unison (EXE5)

- **S1. Chaos Unison in the navi framework**:
  - `ChaosCharge` (actor.rs:393, :416);
  - its window cycling every tick while armed (`chaos_cycle`, input.rs:31; the `chaos_cycle` rule, rules.rs:671);
  - the charge's release (input.rs:284, :354), `chaos_success` and `chaos_failure` (idle.rs:160–305);
  - the glow (charge_glow.rs:99–161; `SpriteRole::ChargeGlowChaos`);
  - the palette (player/mod.rs:1347);
  - `ActorField::{ChaosArmed, ChaosLevel}` and `ActionRole::ChaosFailure`.

  *(b)* EXE5's souls system: a `navi_tick` while armed (§2.1 rule 4 allows per-tick Luau while a state is active), a
  charge-release hook, and the glow's sprite as a look the rules push. About 150 lines. It costs one call per tick
  for an armed navi, and none in a plain fight.
- **S2. Soul data in the engine's records**: `SoulData` and `FormData::soul` (navis.rs:284, :305), `NaviForms::souls`
  (navis.rs:213; defs.rs:1526), which Chaos Unison's Rust (S1) reads. *(a)* With S1's move: the soul list as a form
  list like `by_version`, the soul's data as EXE5's form extension (`SystemDef::extends`). (The transform record's
  names went in step 1: its `alternate`, `form_change_terms`.)

- **S3. The renderer's soul icons and flights** (`soul_icon`, `soul_place`, `soul_flight`, `soul_palette_row`,
  `CHAOS_ICON`, render custom.rs:393–505; they read `NaviForms::souls`). *(c)* The form offer view's icon (§4.8's
  `offer`, `offer_chaos`). Rename to the view.

### The stat block and versions

- **V2. The version-named renderer helpers** (`beast_pictures`, render custom.rs:578; `version_name`,
  `console_version`, `known_emblem`, :592–640, which read `PlayerFact::Version`). Versions themselves are generic:
  a pack's per-version pictures. *(c)* Rename `beast_pictures` to the button's version pictures; nothing else.

### Tools (nettai-match)

- **T3. The stats pane and navi views** (stats.rs: every `NaviStats` field by name; link_navis.rs; story.rs). The
  game's own are listed from its rules' `stats` since V1 (`stats::game_fields`). *(c)* The engine's by role.

### Done

Each with what it was and what it is now.

- **B1.** The `berserk` rule section (`BerserkRules`) is gone. EXE6's rules/berserk.luau is a record of type
  `berserk`, which its berserk controller reads and gen-content checks against the ROM.
- **B3.** Beast Out's turns are EXE6's own stat `beast_out_counter` (V1's way), 3 fresh (its `fresh_stats`), which
  rules/beast spends and rules/emotion reads; the HUD reads the count by its role (`StatRole::WindowCount`,
  `schema.role("window_count", "u8")`), and exe6-compat maps NaviStats+0x21 to it.
- **B4.** `Library::beast_out_chip` is gone. `ChipRole::BeastOut` is `ChipRole::ButtonChip` (`button_chip`): the chip
  a button's pick stands as in the hand, whose picture is the button's.
- **B5, and C6's sounds.** `ScreenSound::{BeastOutGregar, BeastOutFalzar, BeastOutFlash, CrossWindowOpen,
  CrossWindowClose, CrossChosen}` and their `SoundRole::Custom*` roles are gone. The rules play their own sounds in
  the screen's order (`custom.play_sound`, `ScreenSound::Rules`): EXE6's beast/custom.luau and cross/window.luau
  hold them.
- **B7.** `FormEffects::BERSERK` (`berserk`) is `CONTROLLER_RESTART` (`controller_restart`).
- **B8.** `ActionRole::DustBeastScatter` is `Ungrounded` (`ungrounded`), and `ChargeTackle` is `Glowless`
  (`glowless`).
- **B9, C1.** The identity classes are `virus`, `field_object`, `navi` and `player`, and an identity's traits say
  the rest: `changes_form`, `breaks_on_weakness`, `quiet_win`, `bug_blind_immune` and `marker_flat_anim`.
  Where content asked for the class `cross`, EXE6's attachment kind asks whether the identity is one of its navi's
  Crosses' (its form lists'): the lift is that kind's own, as EXE5's is (exelib's attachment has no game's lift).
- **C2.** The form break: `status::FORM_BREAKING`, `breaking_form`, `break_form`, `FormBreak::MarkedForms`
  (`marked_forms`: forms whose identity `breaks_on_weakness`) and `NaviState::FormBreaking` (`form_breaking`).
- **C3.** The navi switch: `switching_navi` and `NaviState::{SwitchingNavi, SwitchKnockout, Switched}`.
- **C4.** `NaviData::cross_hp` is `switch_hp`.
- **C5.** `cross_doubles` and `Boost::Cross` are `charge_doubles` and `Boost::Charged`.
- **C6.** The form list window: `ScreenLook::{form_list_tab, form_list_cursor}` and `custom.draw_form_list_cursor`,
  `custom.set_form_list_tab`.
- **C9.** `PlayerFact::CrossList` is `PlayerFact::FormList`, which a setup field takes by declaring the role
  (`schema.role("form_list", T)`): EXE6's `crosses`, whose name stays its own, so its match files keep
  `crosses = [...]`. `Defs::fact_name` gives the field that holds a role.
- **C10.** The navi's forms table lists a version's forms under `<version>.form_list`.
- **D1.** The dark chip's hover is the shade a dark chip casts: `ChipShade` (`Shading`, `Shaded`),
  `on_shading_chip`, `FadeMode::{Shade, ShadeBack, ShadeWindow, ShadeWindowBack}`, `ScreenSound::Shade` and
  `SoundRole::CustomShade` (`custom_shade`). `ChipFlags::DARK` stays: a chip category of the series, as Mega and
  Giga are, which both games' rules read.
- **D2.** `dark_substitute` is `rules_substitute`.
- **D3.** `BattleInfo::{NoDarkChips, LightDarkHeld}` and `effects::{NO_DARK_CHIPS, LIGHT_DARK_HELD}` are
  `BattleInfo::Effects` (`battle.effects()`), whose bits EXE5's rules/light_dark names.
- **E6.** EXE5's last stand is its rules' `hp_emptied(side, navi)` hook (EXE5's rules/emotion/dark_survival: the check
  0x0802C16C, the hold at 1 HP and the volley's request), which a loss of HP to 0 asks under `status.hp_loss =
  "gauge"` (was `gauge_and_last_stand`), an object with actor data's, and whose answer says whether the hit shows.
  `LastStand`, `last_stand` and `hold_last_stand` are gone; the HP loss is `lose_hp_and_gauge` and
  `apply_damage_shown_by_hp` (X2's names before). A Luau call per navi whose HP reaches 0, only in EXE5.
- **N2.** The bugs a hit inflicts are each game's rules' (@exelib/navicust/bugs, which a game's
  rules/navicust/bugs hands its part): `navi_bug` takes the NaviCust code table (a code below 0x64 names a stat byte,
  written by the stat's name, the game's own by its `stats`' names; the drains add, by EXE5's flags or EXE6's sum;
  the codes from 0xF5, the game's own bugs), and answers "edited" or "spared"; `hit_bug` the HP bug's codes and the
  paralyzing, blinding one, for any navi; `bug_mark` the HP bug's marker; `navi_damaged` the hit bug as damage lands.
  The engine asks them only on a tick a hit landed or brought a code. `set_byte_by_bug_code`, `drain_bug_flags`, the
  hit bug's status roles, the uninstall's spark role and the identity's `bug_blind_immune` (which the rules read)
  leave Rust; the API gains `strip_body_programs`, `refresh_form_flags`, `take_status`, the collision's
  `status_final` and the stats a byte names (`base_form`, `auto_step`, `starting_damage`, the folders', the A
  weapons). exe6-compat's tests/bug_codes.rs checks EXE6's table byte for byte against the codec.
- **N4.** NumbrOpn left the framework's hand size (EXE6's rules/cross deals its ten, with ChargeCross's chips; the
  framework's is the custom level and the hand-shrink bug), and ChpShufl's and NumbrOpn's flags are EXE6's own stats
  (V1's way: `chip_shuffle`, `number_open`).
- **N5** (step c3b). The NaviCust section, `NaviCustRules` and navicust.rs are gone; the board is the game's
  rules/navicust/board.luau, which the editor reads as data.
- **N6.** `clear_bugs`, `init_round_state`, `reset_abilities`, `refresh_abilities`, `apply_ability_flags`,
  `low_hp_support`, and the form's `ability_refresh` (was `navicust_refresh`).
- **S2, its record's names.** `TransformRequest::alternate`, `form_change_terms`, `custom_set_form`'s `alternate`
  and `Screen::form_alternate` (were `chaos`, `form_change_soul`, `form_chaos`).
- **T1.** The save importers are the compat crates' (the user: "import code should move out of nettai-match i think
  and into compat"): `exe6_compat::import` and `exe5_compat::import` each give a whole side by field name, from the
  save's decoded bytes; nettai-match depends on no compat crate, and the caller (nettai-demo's `save_import`) imports
  a save into the arena's game, refusing one of another game. No rules hook: the import is the boundary's.
- **T2.** EXE5's auto battle view is gone, by removal: nettai-match's view, the learner and its exe5_compat call
  (the user: "i don't think you need learning right? since the battles are one-off"). A random EXE5 match states no
  auto battle data; the editor's pane reads the two facts by field name, laid out by EXE5's rules/auto_battle/block
  (`Battle::module_data`), and the save import writes the facts from the block.
- **T4** (step c3b). `has_navicust`, `has_patch_cards` and `navicust_rules` are gone.
- **T5.** `PlayerFact::SpTimes` and `Facts::{sp_times, set_sp_times, takes_sp_times}` are gone. The times are the
  field `sp_times`, which tools set and read through the generic facts and compat writes by name
  (`exe6_compat::codec::SP_TIMES`); the editor's times view keys on a list of `{ <definition>, frames }` and titles
  the field `sp_times` by its name. `RegularChip` and `TagChips` stay roles: the battle folder's deal reads them.
- **V1.** A game's stats of its own are its rules' `stats` (a schema, as `state` and `setup` are): `NaviStats::game`,
  an inline block of 16 bytes at most (`SmallBlock`, copied with the stats), which Luau reads and writes by name
  through `battle.navi(side)` (`CoreApi::navi_game_stats_mut`), `fresh_stats` gives fresh values of by name, and the
  compat codecs map to the block's bytes by name. EXE6's: `beast_out_counter`, `sun`, `chip_drops`, `encounters`,
  `chip_shuffle`, `number_open`; EXE5's: `sun`, `chip_drops`, `encounters`, `hub_style`, `soul_turn_bonus`. Their
  `NaviStats` fields and `NaviStat`s are gone. `version` is no stat: it is the side's version fact (EXE6's API
  `exe6.version`, which MstrCros reads; exe6-compat writes +0x20 from it). The stats pane lists the game's own by the
  schema's names (`stats::game_fields`). A bug code writes them by name (N2: a game's rules/navicust/bugs's `own`).
- **X1.** The fades by what they do: `FadeMode::{Flash, FlashBack, HalfOut, HalfOutBack}` (content's `flash`,
  `half_out`, ...; were `SoulFlash*`, `BeastOut*`), and D1's. A `fades` rule section stays §3.3's way for a fade of a
  game's own, which none needs yet.
- **X2.** `flash_timer_first_reactions`, `apply_damage_gauge_and_last_stand` and `lose_hp_gauge_and_last_stand`.
- **X3.** What it named goes with its entry: `berserk` (B1) went; `lockon` (B2), `chaos_cycle` (S1),
  `effects.full_synchro_aura` (E3) and the `emotion` variants (E1) go with theirs.
- **X4.** The obstacles' soldiers are their conversion: `obstacle_arm_conversion`, `obstacle_disarm_conversion`,
  `obstacle_conversion` (Luau `obstacle.arm_conversion`, ...), `effects.obstacle_conversion`,
  `KindRole::ConvertedObstacle` (`converted_obstacle`) and its state field `ranged`. DustMan's junk is the absorbed
  look: `absorbed_look`, `wear_absorbed_look`. `spawn_mode9_objects` and `start_stance_counter` stay: they name the
  original's battle mode 9 and a stance's counter, not a feature.

- **X5.** The obstacles' conversion is EXE5's rules' `obstacle_reaction(side, obstacle)` hook (objects/soldier's: the
  search 0x080CAB02 and the soldier's spawn 0x080CAAE2), which an obstacle's reaction asks outside the dimming and
  past its first action while a side is armed (`obstacle.arm_conversion`, which stays the framework's).
  `conversion_call`, the rule `effects.obstacle_conversion` and the role `KindRole::ConvertedObstacle` are gone. A Luau
  call per obstacle reaction while a side is armed.

The [stated rule](#the-line) still holds as the goal: when B1–B9, C1–C10 and the rest are done, Rust knows no
"beast", no "cross", no "soul" and no "exe5".
