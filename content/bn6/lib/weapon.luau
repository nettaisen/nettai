--!strict
-- What MegaMan's weapon routines share.

local weapon = {}

-- `sub_8012642`: a form attack's damage, `base` plus `step` for each point
-- of the buster's damage (`sub_801265A`), up to 5.
function weapon.charge_damage(navi: Object, base: number, step: number): number
    return base + step * math.min(navi:buster_damage(), 5)
end

return weapon
