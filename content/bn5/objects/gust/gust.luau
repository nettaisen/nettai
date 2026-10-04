--!strict
-- BN5's gust (attack object #0x49, 0x080CEA0C): the shared one (@common/gust/gust, BN6's code) with BN5's sprite
-- (0C-2E) and collision types, and BN5's differences: any first parameter (one that pulls or is unseen) pushes
-- weaker (hit modifier 4, 0x080CEAA2), and no hit stops it (0x080CEB30 tests its hit flags against 0). Wind's and
-- Fan's fans blow it.

local collision = require("../../rules/collision")
local common = require("@common/gust/gust")

local made = common.make {
    id = "bn5:gust",
    sprite = asset.sprite("bn5:gust"),
    self_type = collision.pull,
    target = collision.hits_navis,
    -- (The code after the table at 0x080CEB94: the spawner's `push {lr}; push {r0-r2, r5}; movs r0, #0x49; bl`.)
    past_table = { 0xB427B500, 0xF7342049 },
    weak_by_any_parameter = true,
    no_stopper = true,
}

return { kind = made.kind, spawn = made.spawn }
