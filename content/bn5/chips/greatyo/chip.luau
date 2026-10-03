--!strict
-- GreatYo (BN5 chip 0x154): the Program Advance of three YoYos with consecutive codes (or two VarSwrds and a YoYo):
-- the YoYo throw (@common/yoyo/throw, action 0x4A with subtype 1) sending out GreatYo's controller (./controller),
-- which throws three yoyos, one a row, 4 panels out and 5 spinning rounds (the chip's parameters: the leader Param2 1,
-- the two followers 2).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("../../objects/attachment/attachment")
local controller = require("./controller")
local throw = require("@common/yoyo/throw")
local yoyo = require("../yoyo/yoyo")
local recipe_varswrd = require("../varswrd/chip")
local recipe_yoyo = require("../yoyo/chip")

local LEADER = yoyo.spec { mode = 1, panels = 4, rounds = 5 }
local FOLLOWER = yoyo.spec { mode = 2, panels = 4, rounds = 5 }

return define.chip {
    id = "bn5:greatyo",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes (written by tools/bn5/recipes.py): in the order of its full table (0x08027FC8); the netbattles'
    -- table (0x0802801C) is that table but for its first 21, which only a battle with each side keeping its own
    -- gauge tries.
    program_advances = {
        { order = 20, sequence = { recipe_varswrd, recipe_varswrd, recipe_yoyo }, per_player_gauges_only = true },
        { order = 56, code_run = { chip = recipe_yoyo, count = 3 } },
    },
    action = throw.action {
        id = "bn5:greatyo/action",
        arm = yoyo.arm,
        attachment = attachment.kind,
        throw = function(me: Object, x: number, y: number, z: number, damage: number): Object?
            return controller.spawn(me, x, y, z, damage, LEADER, FOLLOWER)
        end,
    },
}
