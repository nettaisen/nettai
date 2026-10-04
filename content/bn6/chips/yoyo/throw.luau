--!strict
-- YoYo's and GreatYo's throw (the original's action 0x18, `sub_80EC02A`):
-- the shared throw (@exelib/yoyo/throw) with BN6's yoyo arm and yoyos
-- (yoyo.luau). The navi holds out its yoyo arm and throws a yoyo from the
-- panel ahead, 27 pixels up; GreatYo throws a controller that sends three,
-- one on each row. The navi waits until the yoyo (or the controller)
-- clears the navi's first related object, then puts the arm away over 6
-- ticks.

local attachment = require("../../objects/attachment")
local common = require("@exelib/yoyo/throw")
local yoyo = require("./yoyo")

local throw = {}

-- One state for both throws.
throw.STATE = common.STATE

-- The yoyo arm (the original's attachment row 0x0E).
local ARM = attachment.look { sprite = asset.sprite("yoyo-arm"), palette = 0, attach_point = 0x0F }

-- A chip's throw: how its yoyos fly (a GreatYo's throws the controller);
-- `id`, the action's key (its compat key: both chips are records).
function throw.action(spec: { id: string, yoyo: YoyoSpec }): Action
    return common.action {
        id = spec.id,
        arm = ARM,
        attachment = attachment.kind,
        throw = function(me: Object, x: number, y: number, z: number, damage: number): Object?
            if spec.yoyo.great then
                return yoyo.spawn_controller(me, x, y, 0, z, damage, spec.yoyo)
            end
            return yoyo.spawn(me, x, y, 0, z, damage, spec.yoyo, "related")
        end,
    }
end

return throw
