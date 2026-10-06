--!strict
-- A team navi's damage by its side's level (0x0800EBC4): the entry of one of the damage rows (0x0801D74F, seven
-- entries a row) at the level word the init exchange gave the side (0x0800EBD8 reads it; 0x0800EBE0 counts it from the
-- save's story flags, 0x300 on, up to 6): EXE5's save part's `level` (exe5.navi_level), which the engine knows
-- nothing of. A team navi's weapons take their damage from a row as they fire (`damage`); its own chip's damage is a
-- row too (the chips' damage formulas 50 on, 0x0800EAF8: the chip's `damage = navi_level.row({ ... })`, a function of
-- the side, which the round's setup asks once for each side).

local exe5 = require("../rules/api")

local navi_level = {}

-- `row`'s entry at `navi`'s side's level (a side that operates a team navi states it: the round's setup).
function navi_level.damage(navi: Object, row: { number }): number
    local level = exe5.navi_level(navi.alliance)
    if level == nil then
        error("a team navi's side states no level (0x0800EBC4 reads its damage rows at it)")
    end
    local damage = row[level + 1]
    if damage == nil then
        error(`a damage row of {#row} entries read at level {level}`)
    end
    return damage
end

-- A chip's damage by its side's level: `row`'s entry at it; 0 for a side without a level, which operates no team navi
-- and holds none of their chips (the custom screen reads every chip's damage, for the hand it may build).
function navi_level.row(row: { number }): (side: number) -> number
    return function(side: number): number
        local level = exe5.navi_level(side)
        if level == nil then
            return 0
        end
        local damage = row[level + 1]
        if damage == nil then
            error(`a damage row of {#row} entries read at level {level}`)
        end
        return damage
    end
end

return navi_level
