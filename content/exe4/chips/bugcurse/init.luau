--!strict
-- BugCurse (EXE4 chip 0x12F, Blue Moon's): the dimming chips' variant 0x2A, a curse on the enemy navi's panel that
-- infects it with bugs: @exelib/bugcurse (its controller, effect object #0x56, 0x080E81C4; its curse, attack object
-- #0x80, 0x080DD39C) in EXE4's look: the curse's sheet and VDoll's curse's sound (0x141); what keeps it off the enemy
-- navi (0x080E8208: 0x206, invisible, submerged or flashing); its four bugs (0x080DD494: kinds 2, 3, 4 and 6, each as
-- the hit's byte 3 with 0xFF before it, 0x080DD516).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local common = require("@exelib/bugcurse")

local chip: Chip = {
    codes = { "C" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 73,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = common.make {
        controller_id = "bugcurse/controller",
        curse_id = "bugcurse/curse",
        sprite = asset.sprite("bug-curse"),
        sound = asset.sound("curse"),
        untouched = { "invisible", "submerged", "flashing" },
        bugs = { 0x02FF, 0x03FF, 0x04FF, 0x06FF },
        is = collision.breaking,
        hits = collision.hits_navis,
    },
    damage = 0,
}

return { bugcurse = chip }
