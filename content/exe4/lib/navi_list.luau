--!strict
-- BattleState +0x44's list (four entries, by index): the local console's navi, the other console's, and none past
-- them in a netbattle. The Counters' strike reads it from the other side's place (0x080CFE20), LifeSync's controller
-- for the combatant on a panel and the ones it syncs (0x080E89BA, 0x080E8844).

local navi_list = {}

-- Its length.
navi_list.LENGTH = 4

-- Its entry `i` (0 to 3).
function navi_list.at(i: number): Object?
    local here = battle.local_side()
    if i == 0 then
        return battle.player(here)
    elseif i == 1 then
        return battle.player(bit32.bxor(here, 1))
    end
    return nil
end

return navi_list
