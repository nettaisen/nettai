--!strict
-- BN5's energy burst (attack object #0x11, BN6's code: @exelib/energbom/burst), BN5's kind; its look and collision
-- types BN6's: what EnergBom's and MegEnBom's bomb leaves where it lands (`burst.leave`).

local collision = require("../../rules/collision")
local common = require("@exelib/energbom/burst")

return common.make {
    id = "energbom/burst",
    sprite = asset.sprite("energy-burst"),
    sound = asset.sound("energy-burst"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}
