--!strict
-- A team navi's damage by its side's level (0x0800EBC4): the entry of one of the damage rows (0x0801D74F, seven
-- entries a row) at the level word the init exchange gave the side (0x0800EBD8 reads it; 0x0800EBE0 counts it from the
-- save's story flags, 0x300 on, up to 6). A team navi's weapons take their damage from a row; its own chip's damage is
-- a row too (the chips' damage formulas 50 on, 0x0800EAF8: the chip's `damage = { formula = "level", ... }`).

local navi_level = {}

-- `row`'s entry at `navi`'s side's level (a side that operates a team navi states it: the round's setup).
function navi_level.damage(navi: Object, row: { number }): number
    local level = battle.navi_level(navi.alliance)
    if level == nil then
        error("a team navi's side states no level (0x0800EBC4 reads its damage rows at it)")
    end
    local damage = row[level + 1]
    if damage == nil then
        error(`a damage row of {#row} entries read at level {level}`)
    end
    return damage
end

return navi_level
