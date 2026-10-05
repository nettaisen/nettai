--!strict
-- CircGun's gun (the original's attack object #0x89, `sub_80D655C`): the
-- shared gun (@exelib/circgun/gun), EXE6's kind (./shot's look and kind).

local common = require("@exelib/circgun/gun")
local shot = require("./shot")

local gun = {}

gun.make_kind = common.make_kind
gun.spawn_kind = common.spawn_kind

gun.kind = common.make_kind("circgun/gun", shot.look, shot.kind)

-- EXE6's gun.
function gun.spawn(controller: Object, variant: CircGunVariant, x: number, y: number, element: number, damage: number): Object?
    return common.spawn_kind(gun.kind, controller, variant, x, y, element, damage)
end

return gun
