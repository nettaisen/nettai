--!strict
-- BN5's H-Burst shot (attack object #0x9E, 0x080DA516): the shared shot (@exelib/h-burst/burst, BN6's code) with
-- BN5's look: its probe of no type (row 0, where BN6's is row 0x3D, which BN5 gives its sides' bits), a status 0x10
-- hit, the explosions BN5's effect row 0x3C (0x080DFD84: 14-04, the pink flash, animation 0 in palette 2, BN6's row
-- 0x3D).

local collision = require("../../rules/collision")
local common = require("@exelib/h-burst/burst")
local statuses = require("../../rules/status")

local burst = {}

burst.kind = common.make_kind("h-burst/burst", {
    probe = collision.nothing,
    is = collision.attack,
    hits = collision.hits_navis,
    status = statuses.paralyze_90,
    explosion = define.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 },
})

return burst
