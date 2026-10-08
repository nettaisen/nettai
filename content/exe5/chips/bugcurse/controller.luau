--!strict
-- BugCurse's dimming controller and curse (EXE5's effect object #0x56, 0x080E6AC8, and attack object #0x80, 0x080D6534;
-- EXE4's code): @exelib/bugcurse with EXE5's look: the curse's sheet 0C-3F and its sound; what keeps it off the enemy
-- navi (0x080E6B84: 0x80000206, invisible, submerged, flashing or bubbled); its bugs, five (0x080D6644: code and
-- argument), and the operation battle's three (0x080D664E, three ticks).

local collision = require("../../rules/collision")
local common = require("@exelib/bugcurse")
local operation_battle = require("../../lib/operation_battle")

local controller = {}

-- 0x080E6B62: BugCurse's `dimming` hook.
controller.hook = common.make {
    controller_id = "bugcurse/controller",
    curse_id = "bugcurse/curse",
    sprite = asset.sprite("bug-curse"),
    sound = asset.sound("trap-vanish"),
    untouched = { "invisible", "submerged", "flashing", "bubbled" },
    bugs = { 0x0124, 0x1418, 0x1419, 0x04FF, 0x04FE },
    operation_battle = operation_battle,
    operation_bugs = { 0x1218, 0x04F9, 0x0814 },
    is = collision.breaking,
    hits = collision.hits_navis,
}

return controller
