--!strict
-- What MegaMan's weapon routines (`off_80117D4`) share.

local weapon = {}

-- `sub_8012642`: a form weapon's damage, `base` and `step` more per point
-- of the buster's damage (`sub_801265A`), counting at most 5.
function weapon.charge_damage(navi: Object, base: number, step: number): number
    return base + step * math.min(navi:buster_damage(), 5)
end

return weapon
