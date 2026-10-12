--!strict
-- EXE5's NaviCust programs' effects (docs/design/exe5-map.md §15.13): a program is the engine's
-- (`define.navicust_program`, content/exe5/navicust/), its effects the values the shared constructors make
-- (@exelib/navicust/effects, EXE6's too), each the work of the original's handler for the program (the table at
-- 0x0813FB44); rules/navicust (./init.luau) applies a player's programs to the navi's stats as the round is set
-- up. What a program brings when it is bugged is its `bug` (one of `BUGS`), and which programs only one of on the
-- command line works is its `exclusive`.

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
programs.soul_time = effects.soul_time
programs.hp = effects.hp

-- The bugs, in the order the bugs' routine runs them (0x08140008, its table 0x08140054 by bug group and level): a
-- program's `bug` names the one it brings, the colors' count brings `status` or `status-strong`. Each writes the navi's
-- stats by its level, 1 to 3 (./init.luau).
programs.BUGS = {
    -- 1: steps go astray (+0x31 = 1).
    "step",
    -- 2: the emotion swings (+0x24 = 1).
    "emotion",
    -- 3: a step cracks the panel behind (+0x12 = 3), at level 2, 4 or 8 (+0x13).
    "panel",
    -- 4: the custom screen hurts 20, 40 or 80 more (+0x54).
    "custom",
    -- 5: more random encounters (+0x28 = 1).
    "encounter",
    -- 6: Zenny for chips (+0x26 = 1).
    "result",
    -- 7: the buster fires blanks (+0x14 = 4) and charged shots (+0x15 = 2), at every level.
    "buster",
    -- 8: no supports (+0x0D = 0xFF).
    "support",
    -- 9: a hit gives the HP bug (+0x16 = 3).
    "hp",
    -- 10: HubBatc's: the HP programs give half (the maximum the base and half of them, the HP no more than it), which
    -- the HP's routine then undoes (0x0803C13C sets the maximum and the HP whole).
    "hub",
    -- 11: a status at the battle's start (+0x1A = 9): five colors.
    "status",
    -- 12: a stronger one (+0x1A = 10): six or more.
    "status-strong",
}

-- What each bug is where a tool lists a build's bugs (`overview`): the named bugs it brings, each by its name's key
-- in the locales' `bugs`. A bug that a patch card can bring or fix too is named for the card effect that writes the
-- stat the bug writes (`kind`; ../patch_cards' copy), by that effect's key in the locales' `patch_card_effects`,
-- whose line the name spells out:
--
-- - `step`: the step bug's byte (NaviStats +0x31), which the cards' step bug writes (its line "MoveBug");
-- - `emotion`: the emotion bug's (+0x24), the cards' "EmoBug";
-- - `panel`: the panel trail (+0x12 and +0x13), which the cards' trails write: the bug's is the cracked one (no EXE5
--   card brings it; the trail of none is what a card clears the bug with, "PanelBug", the cards' name for it);
-- - `custom`: the custom screen's damage (+0x54), 20, 40 or 80 more by the bug's level, which the cards' "CustDmg"
--   sets (EXE6's `custom` bug is another, the hand's shrinking: no card's);
-- - `buster`: the blank and charged shots (+0x14 and +0x15: 4 and 2), the cards' "BstBlank";
-- - `hp`: the hit status (+0x16 = 3, a hit gives the HP bug), which the cards' hit statuses write (EXE6's card
--   screen calls this one "DmgHPbug"; no EXE5 card has it, and the one that clears it is "DmgBug"). EXE5's `hp`
--   raises no HP drain (EXE6's does: that is the cards' "BatHPbug", which only cards bring in EXE5);
-- - `status`, `status-strong`: the battle start's bug (+0x1A = 9 and 10), the cards' "StatBug" for 1 and for more:
--   one bug, at level 1 and 2.
--
-- The others no card effect writes, and are listed under their own names: `encounter` and `result` (+0x28, +0x26),
-- `support` (+0x0D = 0xFF, where a card's Rush, Beat and Tango set its bits) and `hub`, which halves what the HP
-- programs add.
programs.BUG_NAMES = {
    step = { { name = "step_bug", kind = "step_bug" } },
    emotion = { { name = "emotion_bug", kind = "emotion_bug" } },
    panel = { { name = "panel_trail.crack", kind = "panel_trail" } },
    custom = { { name = "custom_damage", kind = "custom_damage" } },
    buster = { { name = "buster_blanks", kind = "buster_blanks" } },
    hp = { { name = "hit_status.hp_bug", kind = "hit_status" } },
    status = { { name = "status_bug", kind = "status_bug" } },
    ["status-strong"] = { { name = "status_bug", kind = "status_bug" } },
} :: { [string]: { BugName } }

return programs
