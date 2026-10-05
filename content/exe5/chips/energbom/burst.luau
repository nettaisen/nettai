--!strict
-- EXE5's energy burst (attack object #0x11, EXE6's code: @exelib/energbom/burst), EXE5's kind; its look and collision
-- types EXE6's: what EnergBom's and MegEnBom's bomb leaves where it lands (`burst.leave`).

local collision = require("../../rules/collision")
local common = require("@exelib/energbom/burst")

return common.make {
    id = "energbom/burst",
    sprite = asset.sprite("energy-burst"),
    sound = asset.sound("energy-burst"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}
