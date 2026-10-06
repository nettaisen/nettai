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
| Beast Out and Beast Over | 5 | 3 | 1 | | 9 |
| Crosses and the form framework | 8 | | 2 | | 10 |
| Emotions | | 2 | 1 | 3 | 6 |
| Dark chips and light/dark | 3 | | | | 3 |
| NaviCust | 1 | 3 | | 2 | 6 |
| Souls and Chaos Unison | 1 | 1 | 1 | | 3 |
| The stat block and versions | | 1 | 1 | | 2 |
| Tools | 1 | 2 | 2 | | 5 |
| Across features | 4 | | | | 4 |
| **All** | **23** | **12** | **8** | **5** | **48** |

### Beast Out and Beast Over (EXE6)

- **B1. The `berserk` rule section** (`BerserkRules`, content/rules.rs:1047; sections.rs's `SECTIONS`, :551). Rust
  parses it and nothing reads it. EXE6's rules/beast/berserk.luau requires rules/berserk.luau itself. *(b)* Drop the
  section from `SECTIONS` and `Rules`; the module stays a plain table. Trivial; no cost.
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
- **B4. The BeastOut chip's role** (`ChipRole::BeastOut`, roles.rs:551) and `Library::beast_out_chip`
  (custom/library.rs:17, :77), which nothing calls. *(a)* Delete the method. The role is the Beast Out button's chip,
  as §4.8 names a button's picture; the renderer's own art for it is C7's kind.
- **B5. Beast Out's screen sounds** (`ScreenSound::{BeastOutGregar, BeastOutFalzar, BeastOutFlash}`,
  custom/look.rs:246; their names, screen.rs:988; `SoundRole::CustomBeastOut*`, roles.rs:415). *(a)* The custom
  screen plays sounds by a role name the game's sections list. No closed enum of one game's sounds.
- **B6. The HUD's Beast Out count** (render hud.rs:102, :879; `beast_count_shown`, :926, which reads
  `PlayerFact::BeastOut`) and that fact role (views.rs:124), which nothing else reads. *(c)* A HUD view the rules fill
  (a count, and whether it shows). The fact stays EXE6's setup field, with no role.
- **B7. `FormEffects::BERSERK`** ("berserk", navis.rs:649) and the `berserk` locals around `FormTraits::CONTROLLED`
  (battle.rs:1635, :1852). *(a)* Call it `controller_restart`.
- **B8. `ActionRole::DustBeastScatter`** (roles.rs:51). The navi's hover isn't reset while it runs
  (player/mod.rs:1400). *(a)* Make it a flag on the action (it keeps the body raised) instead of a role.
- **B9. The Cybeasts** (`IdentityClass::{Gregar, Falzar}`, `is_cybeast`, identity.rs:24, :78). Where they act:
  - a win over one has no music or banner and fades to white (battle.rs:1911, :1981, :2001);
  - the 0xF6 bug doesn't blind them (intake.rs:535);
  - the target marker skips Gregar's animation 0x4F (target_marker.rs:146).

  Both games have Cybeasts (EXE5's 0x173 to 0x176). *(a)* Identity traits: a quiet win, immune to the bug's blind,
  the marker's unraised animation as data.

### Crosses and the form framework (EXE6; the form break and the navi switch are both games')

- **C1. The player identity classes** (`IdentityClass::{MegaMan, LinkNavi, Cross, Beast, CrossBeast, BeastOver}`,
  `is_player`, `is_form`, identity.rs:26–75). Read by:
  - the weakness break's class test (status.rs:425);
  - core_api.rs:2943;
  - the identity check (defs.rs:818);
  - the HUD (render hud.rs:979).

  *(a)* Traits on identities: a player, breaks on a weakness hit.
- **C2. The form break under the Cross's name** (`status::CROSS_BREAKING`, `cross_lane`, status.rs:66, :398;
  `break_cross`, actions/transform.rs:218; `FormBreak::CrossOrBeast`, rules.rs:505; `NaviState::CrossBreaking` as
  "cross_breaking", api.rs:754). The mechanism is both games' (rule `form_break`). *(a)* Rename to `form_break` and
  `FORM_BREAKING`, with variants by behavior. It runs on the per-tick reaction path and stays Rust.
- **C3. The navi switch's old names** (`changing_cross`, player/mod.rs:863, transform.rs:237;
  `NaviState::{ChangingCross, CrossKnockout, Crossed}`, api.rs:749, core_api.rs:158). *(a)* Rename the Rust names;
  the Luau names are already `switching_navi`, `switch_knockout` and `switched`.
- **C4. `NaviData::cross_hp`** (navis.rs:109, :916), the HP a navi switched to starts with (actions/navi_switch.rs:283).
  *(a)* Call it `switch_hp`.
- **C5. The chip-use doubling and heal under the Cross's name** (`cross_doubles`, `Boost::Cross`, chip_use.rs:576,
  :614; `heal_on_use`, :661). They already read navi and form data (`charge_doubles`, `chip_heals`, `null_bonus`).
  *(a)* Names only. It runs per chip use, and the form data stays data (§2.1 rule 6).
- **C6. The form list window's state and API under the Cross's name**:
  - `ScreenLook::{cross_tab, cross_cursor}`, `draw_cross_cursor` (custom/look.rs:46, :125, :420);
  - `custom_draw_cross_cursor`, `custom_set_cross_tab` (api.rs:1401, :1403; core_api.rs:991, :1001);
  - `ScreenSound::{CrossWindowOpen, CrossWindowClose, CrossChosen}` and `SoundRole::CustomCross*` (look.rs:233;
    roles.rs:404).

  *(a)* Names of the `FormList` view: `form_list_tab`, `form_list_cursor`, and sounds by role name (as B5).
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
- **C9. `PlayerFact::CrossList`** (views.rs:127). Read by:
  - the renderer (C7);
  - nettai-match: the side's list, its checks and `state_own_forms` (facts.rs:312–682), and `picks_live` and the
    random side (pick.rs:68, :163);
  - nettai-frontend's `live_setup` (driver.rs:179).

  *(a)* A `FormList` role (the forms a form list window offers). "crosses" stays EXE6's field name and nothing else.
- **C10. The navi's per-version form lists read under `<version>.crosses`** (defs.rs:1517; `NaviForms::by_version`).
  *(a)* A neutral key such as `form_list`, which touches EXE6's navi data.

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
- **E6. EXE5's last stand** (`last_stand`, `hold_last_stand`, player/mod.rs:427–460, from `exe5_lose_hp`, :477, and
  `exe5_apply_damage`, status.rs:339; `HpLoss::GaugeAndLastStand`, rules.rs:471). A dark MegaMan holds at 1 HP once a
  battle. *(b)* A hook when a player's HP reaches 0. EXE5's rules/emotion/dark_survival.luau already plays the volley.
  About 40 lines, and the hook is rare. The side gauge's drain on HP loss is a rule variant (X2's kind).

### Dark chips and light/dark

- **D1. The dark chip's hover**:
  - `DarkHover` and `ScreenLook::hover` (custom/look.rs:38, :81, :360–392);
  - `on_dark_chip`, `is_dark` and the opening cursor's dark chip (screen.rs:429, :590, :1571);
  - `FadeMode::DarkChip*`, `ScreenSound::DarkHover` and `SoundRole::CustomDarkHover`;
  - the renderer's fades (render custom.rs:206–237);
  - `ChipFlags::DARK` (chips.rs:132) and the library's Dark section (nettai-content library.rs:53).

  The framework's (§4.4), both games. *(a)* Name it by what it does (a flagged chip whose hover darkens the screen).
  It runs per tick on the screen and stays Rust.
- **D2. `dark_substitute`** (chip_use.rs:385). It is already the rules' hook `chip_substitute`. *(a)* Name only.
- **D3. `BattleInfo::{NoDarkChips, LightDarkHeld}`** (api.rs:590, :593): battle effect bits named for EXE5's dark
  chip rule and light/dark value. *(a)* The battle's effect bits by the names a game's rules give them.

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
  `status_guard`, setup.rs:159–162; `apply_navicust_flags`, `init_navicust`, player/mod.rs:1008–1090). *(d)* Navi
  mechanics the framework reads per tick: panels, holes, flinching, deletion. Chips and forms give them too. Rename
  the `navicust` functions.
- **N4. ChpShufl, NumbrOpn and the shrinking hand in the default hand size** (`chip_shuffle`, `number_open`,
  setup.rs:194–196; `hand_size`, screen.rs:1584–1606). *(b)* EXE6's rules/cross already adds to
  `custom.hand_size`. NumbrOpn and the bug move into it, and the two flags into EXE6's rules state. Small; once per
  custom screen.
- **N5. The NaviCust board** (`NaviCustRules`, rules.rs:751; the `navicust` section, sections.rs:584; navicust.rs, 93
  lines). Read by the editor and nettai-match (`has_navicust`, `navicust_rules`, lib.rs:143–157; import_exe5.rs:116).
  *(b)* The game's rules/navicust/board.luau as a view the editor reads (step c3b). Editor-only.
- **N6. Feature-named API and functions**: `clear_navicust_bugs` (api.rs:2034), `refresh_navicust_state`,
  `low_hp_navicust_effect` (idle.rs:309, the series' Tango support). *(a)* `clear_afflictions` and the like.

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
- **S2. Soul data in the engine's records**:
  - `SoulData` and `FormData::soul` (navis.rs:284, :305);
  - `NaviForms::souls` (navis.rs:213; defs.rs:1526);
  - `TransformRequest::{turns, chaos}` (transform.rs:20);
  - `form_change_soul` (api.rs:1844; core_api.rs:2222);
  - `custom_set_form`'s `chaos` (core_api.rs:870).

  *(a)* The transform record's extra fields as the rules' own result fields, the soul list as a form list like
  `by_version`, and the soul's data as EXE5's form extension (`SystemDef::extends`).
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

- **T1. The save importers** (import.rs: EXE6's version, Beast Out, owned Crosses, level and SP times, with fact names
  as strings, 97 lines; import_exe5.rs: karma, souls, the unisons, ExpMemry, auto battle data and the team navi, 166
  lines). *(b)* A game's save is its Luau's (the line above). Each compat crate keeps decoding the bytes; turning the
  save into facts moves to a rules hook (`import_save`, given the decoded save as a table). Tool-only.
- **T2. EXE5's auto battle view** (auto_battle.rs, about 500 lines without tests: its lists for the editor's pane,
  `AutoBattle::learned`, and `of_folder` for random matches). *(b)* The editor's view kinds (step c3b) and EXE5's
  rules; `learned`'s tie-break waits on the user. Tool-only.
- **T3. The stats pane and navi views** (stats.rs: every `NaviStats` field by name, the per-game ones among them;
  link_navis.rs; story.rs). *(c)* Follows V1: the stats by role, and a game's own from its rules state's schema.
- **T4. `has_navicust`, `has_patch_cards`, `navicust_rules`** (lib.rs:143–157), for the editor's panes. *(c)* Step
  c3b replaces them with view kinds the rules declare.
- **T5. The remaining fact roles**: `PlayerFact::SpTimes` (only tools read it: `Facts::sp_times`, sp_times.rs, the
  editor's SP pane), and `RegularChip` and `TagChips` (the deal reads them: `BattleFolder::shuffled_with_tag_pair`).
  *(a)* `RegularChip` and `TagChips` are the series' folder and stay roles. `SpTimes` becomes a view kind for its
  tool.

### Across features

- **X1. The fade table** (`FadeMode`, battle.rs:192–240: `SoulFlash`, `DarkChip*`, `BeastOut*`, `BlackOut` "the
  Gregar and Falzar chips' controllers"; `custom_fade` by name, core_api.rs:817–829; the renderer's layer choice, render
  custom.rs:202–249). *(a)* A `fades` rule section mapping a name to its code, course and level (§3.3). Rust steps a
  fade by its record and the renderer reads the record's layers.
- **X2. Game-named rule variants and functions**: `exe5_reactions` (status.rs:113), `exe5_apply_damage` (:339) and
  `exe5_lose_hp` (player/mod.rs:477), chosen by `Reactions::FlashTimerFirst` and `HpLoss::GaugeAndLastStand`;
  `FormBreak::CrossOrBeast` (C2). *(a)* Name each by the variant's behavior. They run per tick and stay Rust; the
  last stand goes to E6.
- **X3. Rule sections named for a feature**: `berserk` (B1), `lockon` (B2), `navicust` (N5), `chaos_cycle` (S1),
  `effects.full_synchro_aura` (E3) and the `emotion` section's variants (E1). *(a)* What moves takes its section with
  it; what stays is renamed by role.
- **X4. Content-named API**: the obstacles' soldiers (`obstacle_arm_soldiers` and its kin, api.rs:2154; the
  `effects.obstacle_soldiers` rule), DustMan's junk (`junk_look`, `wear_junk_look`, api.rs:2107),
  `spawn_mode9_objects` and `start_stance_counter` (api.rs:1783, :1788). These name a chip or navi, not a game
  feature. *(a)* Roles for what they do, when their content is next touched.

The [stated rule](#the-line) still holds as the goal: when B1–B9, C1–C10 and the rest are done, Rust knows no
"beast", no "cross", no "soul" and no "exe5".
