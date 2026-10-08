--!strict
-- Mine (EXE4 chip 0x062): the dimming chips' variant 0x12, a land mine hidden on a random panel of the other side's
-- area: EXE6's Mine (@exelib/mine: its controller, effect object #0x29, 0x080E4F5C, and its mine, attack object #0x4C,
-- 0x080D6ED4) in EXE4's look: no NameID; sheet 0c-22, its hop's sound 0x123, its blast 0x70 over effect row 0x47
-- (14-13); its collision rows 0x33 and 0x2A. (Its init frees it without leaving its side's field-object slots when no
-- collision record is left, 0x080D6F5C; the shared code leaves them.)
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/mine/controller")
local land_mine = require("@exelib/mine/land_mine")

local MINE = land_mine.make_kind {
    id = "mine/land-mine",
    sprite = asset.sprite("land-mine"),
    hop_sound = asset.sound("hop"),
    blast = new.effect { sprite = asset.sprite("strike-flash"), anim = 0, palette = 0 },
    blast_sound = asset.sound("hit-bomb-1"),
    self_type = collision.mine,
    target = collision.hits_both_sides,
}

local CONTROLLER = common.make { id = "mine/controller", mine = MINE }

local chip: Chip = {
    codes = { "C", "H", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 42,
    damage = 300,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    dimming = CONTROLLER.hook,
}

return { mine = chip }
