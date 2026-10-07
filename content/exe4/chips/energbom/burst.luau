--!strict
-- EXE4's energy burst (attack object 0x11, 0x080CF300: EXE6's `sub_80C67F8`'s code, @exelib/energbom/burst), EXE4's
-- kind: its sprite (14-12), its sound (0xBD), thrown and hitting navis (0x080CF3D8's word): what EnergBom's and
-- MegEnBom's bomb leaves where it lands (`burst.leave`, 0x080CF3B8).

local collision = require("../../rules/collision")
local common = require("@exelib/energbom/burst")

return common.make {
    id = "energbom/burst",
    sprite = asset.sprite("energy-burst"),
    sound = asset.sound("energy-burst"),
    target = collision.hits_navis,
    self_type = collision.thrown,
}
