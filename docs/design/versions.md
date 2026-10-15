# Game versions, builds and rulesets

**Status: decided (2026-10-15), built (2026-10-15: §4).** The user's answers to the proposal's open choices are §3.

The user, as rulesets came in ("allowed sets of souls/crosses (should be able to represent game)"): "i think then
you might want to rethink version selection for saves if you can mix and match souls/crosses. it's kind of
irrelevant then right? though bn6 selects mastercross depending on game version. and i guess how would you
represent version-exclusive gigas/crosses/souls if the save file no longer holds information about what game it
is? i want a design for this", and "i suppose the bass-cross megaman patch card handling would have to make sense,
too".

## 1. What a version decides today

A version (BN4 Red Sun/Blue Moon, BN5 Team ProtoMan/Team Colonel, BN6 Gregar/Falzar) is chosen as a build is
made (EXE5 and EXE6: the setup's `version`; EXE4 already has none in its rules, only presets: the user's
2026-10-07 call, "exe4 implementation should not really know about version, it really is all just save stuff").
What it decides falls in two kinds:

**What a save can own** (a fact about the collection, checked as a build is made):

| | BN4 | BN5 | BN6 |
|---|---|---|---|
| Souls / Crosses | six souls each (presets already) | six souls each (`Version::soul_flag`) | five Crosses each, and its Beast (`gregar-beast`, `falzar-beast`) |
| Giga chips | five each, and Duo either | six each, and ChaosLrd either | five each, and Gregar/Falzar (JP) |

**What changes in a battle** (a fact the rules read):

| | BN4 | BN5 | BN6 |
|---|---|---|---|
| Patch card | none | card 111 Bass-Cross MegaMan: HP+200, SprArmr, the rapid buster, Hub Style 1 (ProtoMan) or HP+20%, FlotShoe, the Hell's Rolling shot, Hub Style 2 (Colonel) | card 117 Bass-Cross MegaMan: the same split (Gregar, Falzar) |
| Beast Out | | | bare MegaMan's Beast is his version's (`rules/beast/custom.luau`) |
| MstrCros | | | the Program Advance plays MegaMan's five Crosses "of his user's game" (`chips/mstrcros/navi.luau`) |
| Crossing in Beast Out | | | only a Cross of the Beast's own version (`rules/cross/window.luau`: each Cross has one Beast form, its version's) |

And one look: the version's tag and color on a build (Team Colonel's green, Gregar's green, Falzar's red).

So outside BN6's Beast, a version decides nothing in a battle but one patch card. Everything else is what a save
could own, and once a ruleset says what may be played, a build needn't be held to one cartridge's collection.

## 2. The proposal

### 2.1 No version on a build

A build states what it has, not which cartridge it came from:

- **Its souls / Crosses:** the navi's form list, any of the game's (today: a version's list, or EXE4's preset).
- **Its folder:** any of the game's chips the folder rules allow, Giga chips of both versions included.
- **BN6's Cybeast:** the one choice left that changes a battle on its own. MegaMan holds one Beast, Gregar or
  Falzar (in the story the version *is* the Cybeast he took in; "they're called "cybeasts", not "cyberbeasts""), stated as a form in his list like the
  Crosses. It decides bare MegaMan's Beast Out and the five Crosses MstrCros plays (the Beast's version's,
  as the original's "his user's game"). Crossing in Beast Out keeps the original's rule, by each form's own
  version (a Cross's Beast form is its version's), so a build with the other version's Crosses can Cross into
  them outside Beast Out and not in it, as each Cross only has its own Beast.
- **Bass-Cross MegaMan:** two cards in effect, so its effects are the card's own choice, not the build's: the card
  has a variant, as a NaviCust program has colors (`colors = { "white" }`): "Team ProtoMan's" or "Team
  Colonel's" (EXE5), "Gregar's" or "Falzar's" (EXE6), each with that version's effects. A build that slots the
  card picks the variant in the editor (the card's chooser, as a program's color is chosen). The one card stays
  one entry, as the user asked on 2026-10-14 ("EXE5's two cards made one"); only which half of it plays moves
  from the build's version to the card.

**The editor chooses them** (the user: "you probably need to have some kind of soul and cross and cybeast selector
in the editor now right?"). The build editor's Navi tab gets a selector for what the version used to choose:

- **Souls (BN4, BN5) and Crosses (BN6):** which of the game's the build has, both versions' offered, in the order the
  battle lists them, as many as the battle's own list holds (the original's window: its count is the game's to
  state, checked against the ROM's, not guessed here), each shown as the Navi tab shows a form today. A ruleset
  chosen in the editor marks the banned ones, as the ruleset work marks banned chips.
- **BN6's Cybeast:** Gregar or Falzar, one always chosen, at the head of the selector (where the version's tag was
  in the header: app-ui §2.42 goes).
- **A new build** starts from a version's group (§2.2): that version's souls or Crosses and its Cybeast, changed
  from there.

The setup's `version` goes from EXE5's and EXE6's rules (EXE4 has none already); `exe6.version(side)` is replaced
by the side's Cybeast where the rules read it (Beast Out, MstrCros).

### 2.2 Versions become groups the content declares

Each game's content still knows its versions, as **groups**: a named, colored set of what a save of that
version can own, declared once, in the game's rules (where `versions` is now):

    groups = {
        { id = "gregar", color = 0x45E0A0, forms = { gregar's Crosses, "gregar-beast" },
          chips = { its Giga chips, "gregar" }, cards = { ["bass-cross-megaman"] = "gregar" } },
        { id = "falzar", ... },
    }

(Shapes are a sketch; the content model's rules hold: by keys, no lookup tables, no fields the content already
states — each chip's and form's own `version` already says which group it is in, so a group may be just its id,
color and name, its members found by that field.)

A group is used for two things:

1. **Rulesets** (the ruleset work under way): "allowed sets of souls/crosses" and the Giga chips' allowlist offer
   each group as one choice (§2.3).
2. **New builds:** "New build" offers each group as a starting point (its souls or Crosses, and BN6 its Beast),
   as EXE4's presets do today.

A build shows no version: no tag, no label. Whether a build fits one version is the ruleset's to say, as a problem
where a ruleset asks for it (the user: "the builds don't have version labels either, they're kind of pointless
since they correspond to ruleset rules"). The header's version tag (app-ui §2.42) goes; on a BN6 build its place
is the Cybeast's choice.

### 2.3 Rulesets represent the cartridges

The "should be able to represent game" part. A ruleset gets one more setting, **Versions**:

- **One per build** (the cartridges): each build must fit one group: its version-only forms, chips, Beast and
  Bass-Cross variant all one version's. Which groups are allowed is the "allowed sets" choice (both, or one).
- **Mixed**: no such check; the allowlists alone say what may be played.

So "BN6 as the game was" is: Versions one per build, both groups allowed, everything else allowed. A Falzar-only
tournament: one per build, Falzar's group alone. An open format: mixed. The per-soul and per-Cross toggles the user
asked for sit under the groups, as the ruleset work already has them.

A build breaking it is the lobby's usual typed problem: "Mixes Gregar's and Falzar's" (listing what), as a banned
chip is.

(The app words it for a player as **Version exclusives**, ONE VERSION | MIX VERSIONS (app-ui §2.65: the user, "one
per build is weird wording"); the ruleset files keep `versions = "one_per_build"` or `"mixed"`.)

### 2.4 Saves, recordings and the original

The version stays where the original keeps it: the save, and the boundary that reads it (exe5-compat, exe6-compat,
as EXE4 does already):

- **A save's import:** the save's version gives the build its forms (that version's owned Crosses or souls, BN6
  its Beast) and the variant of a Bass-Cross MegaMan card it has slotted.
- **Recordings and traces:** the console's version, which the trace harness states, is turned into the same facts
  (Beast, card variant), so every battle the original plays is the one nettai plays: the verify gates see no
  difference.

## 3. Decided

The user, of the four open choices: "1. bass cross variant on the card 2. drop them, ignore backward compatibility
3. no label 4. one per build".

1. **Bass-Cross MegaMan's variant is the card's own** (§2.1), in BN5 and BN6 alike, chosen in the editor as a
   program's color is; BN6's Cybeast doesn't decide it.
2. **Builds saved before this are dropped:** no reading of the old `version`, no conversion; the format changes in
   place (nothing is in production).
3. **No build has a version label** (first "no label" for a build with nothing version-only, then none at all: "the
   builds don't have version labels either, they're kind of pointless since they correspond to ruleset rules").
4. **The built-in "Standard" ruleset's Versions is "one per build"**: each build one cartridge's, both versions
   allowed, as the original played.

## 4. As built (2026-10-15)

- **The groups** are each game's rules/builds `groups` (`{ id, color }`, named `builds.version.<id>`), in the order
  every list shows them. Their members are found by the definitions' own `version` (nettai-match's
  `ruleset::Rulesets`): EXE6's Crosses, Beast forms and Giga chips said theirs already; EXE4's and EXE5's souls (the
  engine's form field) and Giga chips (a `version` each game's rules now extend chips with) say theirs too; ChaosLrd
  and Duo say none. A Bass-Cross MegaMan card's variant is the group of its name. The versions' `set` records,
  EXE6's `shown_first` and the place-in-the-list reading of an EXE5 soul's version (the render's soul icons now read
  the form's `version`) are gone.
- **No version on a build.** EXE5's setup has no `version`; EXE4's and EXE5's `giga_chips` and the folder rules'
  `owned` (the `chip_of_another_version` problem, nettai-match's `Wrong::ChipOfAnotherVersion`) are gone: a folder
  may hold either version's Giga chips, and a ruleset of one version per build refuses a build that mixes them.
- **EXE6's Cybeast** is the setup's `cybeast`, the engine's version fact as a form (`schema.role("version",
  "form")`: the side's version is the form's own `version`, `Battle::version`; a side states it or the round
  doesn't start, as an enum's). Its forms are MegaMan's sets' `cybeast` (the Beast Out forms, Gregar's and
  Falzar's: `facts::offered_forms`). Beast Out from the base form takes the navi into the Cybeast, Beast Over and
  the roar are the Cybeast's version's (rules/beast/custom.luau), and MstrCros plays the Cybeast's version's five
  Crosses (`exe6.cybeast(side)` in place of `exe6.version`). The custom screen's version pictures go by it too. Its
  ruleset category is `cybeast`.
- **The counts.** EXE6's Crosses: `form[5]`, the Cross window's (`sub_8029EC8` counts five flags of
  `byte_8029F4C`). EXE5's souls: `form[12]` (it was 16), the soul button's list (0x08024B28 goes through twelve
  families of 0x08024BE0, `cmp r6, #13`, with their flags at 0x08024BF0: Team ProtoMan's ROM's six and six 0xFF).
  EXE4's: `form[12]` as before (0x08020018's twelve flags after 0x08020008's twelve families).
- **The card's variant** is its setup record's: EXE5's and EXE6's `patch_cards` are lists of `{ card, variant }`,
  `variant` an enum of the game's versions that a card without variants leaves out (the engine leaves a record's
  enum that a fact leaves out unstated; the rules see it as nil). A card's variants are the versions its effects
  name (`version`, as before); @exelib/patch_cards/apply runs each installed card with its own variant.
- **The compat boundary** (§2.4): exe6-compat's save import and trace setup state the save's version's Cybeast
  (`codec::cybeast`) and each card's variant of the save's version (`codec::patch_card`); exe5-compat's the card's
  variant (the version fact is gone); the version byte NaviStats+0x20 is compared as the Cybeast's
  (`GameVersion::stats_byte`). The app's import keeps the forms the save owns.
- **Random matches** (nettai-match's `pick`): a picked side states a random Cybeast where the rules require one
  (`pick::Required`), from the same stream as the version before; random folders may hold either version's Giga
  chips.
- **EXE5's soul icon** (the soul button's icon as it flies to the picked column, sprite palette 13: Team Colonel's
  ROM's outline, color 9, is another than Team ProtoMan's, 0x0874BDBC against 0x0874AAB8). The original loads its
  cartridge's palette (state 9, 0x0802330C), so every soul's icon is in the console's version's outline. With no
  version on a build the icon follows **the soul it shows**: the soul offered (a Soul Unison's, a Chaos Unison's)
  and, for a MeddySoul capsule's mix, the soul the side is in, each in its form's own `version`'s outline (nettai-
  render's `icon_palette_row`, the pack's `icon_versions`). Why that rule: a cartridge holds its own team's souls
  alone, so for every build the original can make the soul's version is the cartridge's and the icon is the
  original's on its own console; it needs nothing a build doesn't state already (no field: the form's `version`
  is the one a ruleset's groups read), and a mixed build (a Mixed ruleset's) shows each soul as its own cartridge
  would, where a rule by the souls the build holds would have to pick one team's outline for both. Rust names no
  soul or version: the render reads the form's `version` and the pack's palettes by that name, and no console's
  version (a recording's names one only to mark the other team's soul on it as known, `OTHER_VERSIONS_ICON`), so
  a live battle draws what a recording does. Checked frame for frame against mGBA (2026-10-15, chip lab's
  souls/07-obstacle/unison-own-console, ColonelSoul on a Team Colonel console, and souls/01-sword/unison,
  ProtoSoul on Team ProtoMan's: exact but at the replay's known shift, the flight's frames among them, its outline Team Colonel's gray and Team ProtoMan's
  teal; souls/06-recovery/capsule-soul, MeddySoul's capsule, exact but at the replay's known shift).
- **Play's build list and Practice** check builds under a ruleset (`Checked::under`'s problems: nettai's
  `builds::shelf`), as the lobby and the build editor do: outside a lobby the one the player plays by, so a build
  that mixes versions under Standard is "2 problems" in the list and PRACTICE and HOST refuse it with the first
  ("Mixes Gregar and Falzar: …"); in a lobby CHANGE BUILD's list is checked again under the lobby's, the host's,
  as it changes. Joining (an invite's code, an address) asks only the game's rules of a build, as the lobby it
  joins is under the host's ruleset and checks the build under it.
