--!strict
-- BN5's energy burst (attack object #0x11, BN6's code: @common/energbom/burst), BN5's kind; its look and collision
-- types BN6's: what EnergBom's and MegEnBom's bomb leaves where it lands (`burst.leave`).

local bn6_collision = require("@bn6/rules/collision")
local common = require("@common/energbom/burst")

return common.make {
    id = "bn5:energbom/burst",
    sprite = asset.sprite("bn6:energy-burst"),
    sound = asset.sound("bn6:energy-burst"),
    target = bn6_collision.hits_navis,
    self_type = bn6_collision.thrown,
}
