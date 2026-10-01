--!strict
-- What the local player sees (presentation that reaches the objects'
-- visibility flags, which the traces compare).

local viewer = {}

-- `sub_800EB6C`: whether the local player sees objects of `side`: always
-- its own side's; the other side's unless its own navi is blind.
function viewer.sees(side: number): boolean
    if side == battle.local_side() then
        return true
    end
    local navi = battle.player(bit32.bxor(side, 1))
    return not (navi and navi:status("blind"))
end

return viewer
