--!strict
-- EXE6's NaviCust programs' effects (docs/design/navicust.md): a program is
-- the engine's (`define.navicust_program`, content/exe6/navicust/), its
-- effects the values the shared constructors make (@exelib/navicust/effects,
-- which EXE5's programs share), each the work of the original's handler for
-- the program (`navicust_jt_NCPs`); rules/navicust (./init.luau)
-- applies a player's programs to the navi's stats as the round is set up.
-- What a program brings when it is bugged is its `bug` (one of `BUGS`), and
-- which programs only one of on the command line works is its `exclusive`.

local effects = require("@exelib/navicust/effects")

local programs = {}

export type Effect = NaviCustEffect

programs.super_armor = effects.super_armor
programs.custom = effects.custom
programs.mega_folder = effects.mega_folder
programs.giga_folder = effects.giga_folder
programs.first_barrier = effects.first_barrier
programs.back_special = effects.back_special
programs.float_shoes = effects.float_shoes
programs.air_shoes = effects.air_shoes
programs.undershirt = effects.undershirt
programs.chip_shuffle = effects.chip_shuffle
programs.number_open = effects.number_open
programs.collect = effects.collect
programs.outside = effects.outside
programs.attack = effects.attack
programs.rapid = effects.rapid
programs.charge = effects.charge
programs.attack_max = effects.attack_max
programs.rapid_max = effects.rapid_max
programs.charge_max = effects.charge_max
programs.bug_stop = effects.bug_stop
programs.rush = effects.rush
programs.beat = effects.beat
programs.tango = effects.tango
programs.hp = effects.hp

-- The bugs, in the order the bugs' routine runs them (`sub_813CBCC`, its
-- table `byte_813CC18` by bug and level): a program's `bug` names the one
-- it brings, the colors' count brings `status` or `status-strong`. Each
-- writes the navi's stats by its level, 1 to 3.
programs.BUGS = {
    -- 1: steps go astray (+0x31 = 1).
    "step",
    -- 2: the emotion swings (+0x24 = 1).
    "emotion",
    -- 3: a step cracks the panel behind (+0x12 = 3), at level 2 to 4 (+0x13).
    "panel",
    -- 4: from custom screen 4, 3 or 2 on, the hand shrinks (+0x63).
    "custom",
    -- 5: more random encounters (+0x28 = 1).
    "encounter",
    -- 6: Zenny for chips (+0x26 = 1).
    "result",
    -- 7: the buster fires 6, 10 or 13 blanks of 16 (+0x14) and 1, 2 or 3
    -- charged shots (+0x15).
    "buster",
    -- 8: no supports (+0x0D = 0xFF).
    "support",
    -- 9: the HP drains 1, 2 or 3 levels more (+0x18), and a hit gives the HP
    -- bug (+0x16 = 3).
    "hp",
    -- 10: NaviStats+0x62 = 3, 2 or 1, which no battle reads. No program
    -- brings it.
    "unread",
    -- 11: a status at the battle's start (+0x1A = 9): five colors.
    "status",
    -- 12: a stronger one (+0x1A = 10): six or more.
    "status-strong",
}

-- What each bug is where a tool lists a build's bugs (`overview`): the named bugs it brings, each by its name's key
-- in the locales' `bugs`. A bug that a patch card can bring or fix too is named for the card effect that writes the
-- stat the bug writes (`kind`; ../patch_cards' copy, docs/engine/patch-cards.md §2.1's effect numbers), by that
-- effect's key in the locales' `patch_card_effects`, whose line the name spells out:
--
-- - `step`: +0x31, the step bug, which effect 0x83 writes (its line "MoveBug");
-- - `emotion`: +0x24, which effect 0x95 writes ("EmoBug");
-- - `panel`: the panel trail, +0x12 and +0x13, which effects 0x84 to 0x8D write, one a trail: the bug's is the cracked
--   one, 0x84's ("MoveCrak"; 0x8D's none is what a card clears the bug with, "PanelBug", the cards' name for it);
-- - `buster`: the blank and charged shots, +0x14 and +0x15, which effect 0x8E writes ("BstBlank");
-- - `hp`: two bugs. It raises the HP drain by its level, +0x18, which effect 0x96 adds to or clears ("BatHPbug");
--   and it sets the hit status, +0x16 = 3 (a hit gives the HP bug), which effect 0x92 writes ("DmgHPbug") and 0x93
--   clears. A card that clears one leaves the other;
-- - `status`, `status-strong`: the battle start's bug, +0x1A = 9 and 10, which effect 0x98 writes for 1 and for more
--   ("StatBug"): one bug, at level 1 and 2.
--
-- The others no card effect writes, and are listed under their own names: `custom` shrinks the hand (+0x63; the
-- cards' "CustDmg" and "CusHPbug" are the custom screen's damage and HP drain, +0x54 and +0x19, other bugs, which
-- only cards bring in EXE6), `encounter` and `result` are +0x28 and +0x26, and `support` empties the supports
-- (+0x0D = 0xFF, where a card's Rush, Beat and Tango set its bits).
programs.BUG_NAMES = {
    step = { { name = "step_bug", kind = "step_bug" } },
    emotion = { { name = "emotion_bug", kind = "emotion_bug" } },
    panel = { { name = "panel_trail.crack", kind = "panel_trail" } },
    buster = { { name = "buster_blanks", kind = "buster_blanks" } },
    hp = { { name = "hp_drain", kind = "hp_drain" }, { name = "hit_status.hp_bug", kind = "hit_status" } },
    status = { { name = "status_bug", kind = "status_bug" } },
    ["status-strong"] = { { name = "status_bug", kind = "status_bug" } },
} :: { [string]: { BugName } }

return programs
