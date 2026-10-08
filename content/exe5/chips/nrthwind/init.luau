--!strict
-- NrthWind (EXE5 chip 0x08a): a north wind blows over the field for 100 ticks, then strips every barrier off it: the
-- dimming handler's subtype 21 (0x080E4854), whose controller (effect object #0x39, 0x080E4804), wind (attack object
-- #0x5A, 0x080D122C) and streaks (effect object #0x3A) are EXE6's code that no EXE6 chip uses (`sub_80E47D4`,
-- `sub_80CFC08`, `sub_80E4848`: @exelib/nrthwind/controller and wind): a streak every 8 ticks somewhere along the
-- user's back edge, and at the end the wind's whole-field region (row 0x1B, which wind hits remove barriers by).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/nrthwind/controller")
local north_wind = require("@exelib/nrthwind/wind")

-- EXE5's north wind: its sound (0x0AD, dustbrk), its streaks' sheet (EXE5's 0x080E4898 loads its own, 0c-2e, a gust
-- of white wind, where EXE6's leftover code names 0c-00), and the spawn's side and flip (EXE6's `sub_80CFCD6`).
local WIND = north_wind.make {
    id = "nrthwind",
    sound = asset.sound("dustbrk"),
    streak_sprite = asset.sprite("gust"),
    is = collision.collision_1b,
    hits = collision.nothing,
    copies_flip = true,
}

-- The controller (effect object #0x39, EXE6's `sub_80E47D4`): the wind (of the controller's side and facing, running
-- while dimmed); once it is done, undim. Its spawn copies the user's facing with its side.
local DIMMING = controller.make { id = "nrthwind/controller", wind = WIND, side_only = false }

local chip: Chip = {
    codes = { "D", "Q", "V", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    dimming = DIMMING,
    auto_battle = { class = "when_enemy_barrier" },
}

return { nrthwind = chip }
