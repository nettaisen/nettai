--!strict
-- What MegaMan's weapon routines (`off_80117D4`) and their actions share.

local weapon = {}

-- `sub_8012642`: a charged weapon's damage, `base` plus `step` for each
-- point of the buster's damage (`sub_801265A`), counting at most 5.
function weapon.charge_damage(navi: Object, base: number, step: number): number
    return base + step * math.min(navi:buster_damage(), 5)
end

-- `sub_80B8E58(AIData+0x68)`: the buster arm in the navi's overlay slot
-- ends (its lifecycle word goes to destroy). The slot keeps the handle.
function weapon.end_arm(navi: Object)
    local arm = navi.overlay
    if arm then
        arm:set_lifecycle("destroy")
    end
end

-- A move cutting a shot short (`object_canMove`, `sub_800FA54`,
-- `sub_800F964`): the direction to step in, if the navi can.
function weapon.move_input(navi: Object): number?
    if not navi:can_move() then
        return nil
    end
    local dir = navi:held_direction()
    if dir == 0 or navi:step_target(dir) == nil then
        return nil
    end
    return dir
end

return weapon
