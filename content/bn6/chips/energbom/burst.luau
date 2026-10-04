--!strict
-- The energy burst (the original's attack object #0x11, `sub_80C67F8`):
-- the shared burst (@exelib/energbom/burst), BN6's kind and look: what
-- EnergBom's and MegEnBom's bomb leaves where it lands (`burst.leave`).

local collision = require("../../rules/collision")
local common = require("@exelib/energbom/burst")

return common.make {
    id = "bn6:energbom/burst",
    sprite = asset.sprite("bn6:energy-burst"),
    sound = asset.sound("bn6:energy-burst"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}
