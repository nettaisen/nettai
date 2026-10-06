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
| Beast Out and Beast Over | | 2 | 1 | | 3 |
| Crosses and the form framework | | | 2 | | 2 |
| Emotions | | 2 | 1 | 3 | 6 |
| NaviCust | | 2 | | 2 | 4 |
| Souls and Chaos Unison | 1 | 1 | 1 | | 3 |
| The stat block and versions | | 1 | 1 | | 2 |
| Tools | | | 1 | | 1 |
| Across features | | 1 | | | 1 |
| **All** | **1** | **9** | **7** | **5** | **22** |

Of the audit's 48, step 1 (2026-10-06) did every name (kind (a)) but S2's part that waits on S1, and B1 and B4's
dead code; the library agent's steps did N5, T2 and T4, and the move of the importers into compat T1. They are listed under [Done](#done) with their new names.
X5 is new: what X4's rename left of EXE5's logic.

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
- **B3. Beast Out's turns** (`NaviStats::beast_out_counter`, setup.rs:167; the bug-code writer, :359;
  `fresh_stats.beast_out_counter`, rules.rs:326; `NaviStat::BeastOutCounter`, api.rs:480). *(b)* A field of EXE6's
  rules state, which rules/beast already spends and rules/emotion reads. The HUD's count becomes a view (B6). Small:
  read at a turn's start and a custom screen.
- **B6. The HUD's Beast Out count** (render hud.rs:102, :879; `beast_count_shown`, :926, which reads
  `PlayerFact::BeastOut`) and that fact role (views.rs:124), which nothing else reads. *(c)* A HUD view the rules fill
  (a count, and whether it shows). The fact stays EXE6's setup field, with no role.
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
- **E6. EXE5's last stand** (`last_stand`, `hold_last_stand`, player/mod.rs:427–460, from `lose_hp_gauge_and_last_stand`,
  :477, and `apply_damage_gauge_and_last_stand`, status.rs:339; `HpLoss::GaugeAndLastStand`, rules.rs:471). A dark MegaMan holds at 1 HP once a
  battle. *(b)* A hook when a player's HP reaches 0. EXE5's rules/emotion/dark_survival.luau already plays the volley.
  About 40 lines, and the hook is rare. The side gauge's drain on HP loss is a rule variant (X2's kind).

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
- **N2. Inflicting bugs** (`bug_navicust`, `bug_hp_level`, `bug_paralyze_blind`, `strip_programs`,
  `navicust_hit_bug`, intake.rs:503–730; the hitbox's `bug`, hitbox.rs:22; collision.rs:137, :571, :727–740;
  `NaviStats::set_byte_by_bug_code`, setup.rs:283–400). These are EXE6's bug codes (0x18, 0xF5 to 0xFE) and its table
  from code to NaviStats offset. *(b)* A hook `bug_inflicted(side, code, arg)` with EXE6's table from code to stat in
  its Luau, writing through `NaviStat` (§3.3). About 310 lines. It runs per hit that carries a bug, which is rare.
- **N3. The abilities in the stat block** (`float_shoes`, `air_shoes`, `undershirt`, `super_armor`,
  `status_guard`, setup.rs:159–162; `apply_ability_flags`, `init_round_state`, player/mod.rs:1008–1090). *(d)* Navi
  mechanics the framework reads per tick: panels, holes, flinching, deletion. Chips and forms give them too. (Their
  functions' `navicust` names went in step 1: N6.)
- **N4. ChpShufl, NumbrOpn and the shrinking hand in the default hand size** (`chip_shuffle`, `number_open`,
  setup.rs:194–196; `hand_size`, screen.rs:1584–1606). *(b)* EXE6's rules/cross already adds to
  `custom.hand_size`. NumbrOpn and the bug move into it, and the two flags into EXE6's rules state. Small; once per
  custom screen.
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

- **V1. The stat block's other per-game fields** (`NaviStats`, setup.rs): `version` (EXE6's version byte, :165, set
  at battle.rs:789), `sun` (EXE6's, :169), `chip_drops` and `encounters` (:173), `hub_style` (EXE5's, :202),
  `soul_turn_bonus` (EXE5's, :207), and their `NaviStat`s. B3 and N4 cover `beast_out_counter`, `chip_shuffle` and
  `number_open`. They mirror the original's block, which compat and the traces compare. *(b)* Fields of each game's
  rules state, with compat mapping them to the block's offsets. Small each; read at round setup and a few chip uses.
- **V2. The version-named renderer helpers** (`beast_pictures`, render custom.rs:578; `version_name`,
  `console_version`, `known_emblem`, :592–640, which read `PlayerFact::Version`). Versions themselves are generic:
  a pack's per-version pictures. *(c)* Rename `beast_pictures` to the button's version pictures; nothing else.

### Tools (nettai-match)

- **T3. The stats pane and navi views** (stats.rs: every `NaviStats` field by name, the per-game ones among them;
  link_navis.rs; story.rs). *(c)* Follows V1: the stats by role, and a game's own from its rules state's schema.

### Across features

- **X5. The obstacles' conversion** (`conversion_step`, `conversion_call`, kinds/obstacle.rs:525–620): EXE5's
  ColonelSoul army's step in an obstacle's reactions (0x080CAB02), which looks for where an armed side can use the
  obstacle and turns it into the side's converted obstacle (the role `converted_obstacle`, its `ranged` state set by
  the engine). Renamed in step 1 (X4); the search is still EXE5's logic. *(b)* A hook on an obstacle's reaction in
  EXE5's rules, with the search in Luau. About 100 lines; it runs on an obstacle's reaction while a side is armed.

### Done

Each with what it was and what it is now.

- **B1.** The `berserk` rule section (`BerserkRules`) is gone. EXE6's rules/berserk.luau is a record of type
  `berserk`, which its berserk controller reads and gen-content checks against the ROM.
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

The [stated rule](#the-line) still holds as the goal: when B1–B9, C1–C10 and the rest are done, Rust knows no
"beast", no "cross", no "soul" and no "exe5".
