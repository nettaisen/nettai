--!strict
-- A team navi's damage by its side's level (0x0800EBC4): the entry of one of the damage rows (0x0801D74F, seven
-- entries a row) at the level word the init exchange gave the side (0x0800EBD8 reads it; 0x0800EBE0 counts it from the
-- save's story flags, 0x300 on, up to 6): the rules' `level` (exe5.navi_level), which the engine knows
-- nothing of. A team navi's weapons take their damage from a row as they fire (`damage`); its own chip's damage is a
-- row too (the chips' damage formulas 50 on, 0x0800EAF8: the chip's `damage = navi_level.row({ ... })`, a function of
-- the side, which the round's setup asks once for each side).
--
-- A side with partners (EXE5 DS's party battle, rules/party) takes the DS's damage instead, at its level 0 to 9: a
-- weapon's its row of the DS's link table (0x0203C044, 0x0203E31E: a base and a step by the level), the base and a step
-- for each attack level of the navi on the field and one (PartyCust's P.Atk raises it); a team navi's own chip's its
-- navi's row (0x0203BF2C, through 0x02004E48: the navi's `party.chip_damage`) and its member's chip attack bonus
-- (PartyCust's P.Chp), 999 when that navi isn't in the party.

local exe5 = require("../rules/api")

local navi_level = {}

-- `row`'s entry at `navi`'s side's level (a side that operates a team navi states it: the round's setup); in a party,
-- `party`'s (its base and its step) by the navi's attack level.
function navi_level.damage(navi: Object, row: { number }, party: { { number } }?): number
    local side = navi.alliance
    if party and exe5.in_party(side) then
        local pair = party[(exe5.navi_level(side) :: number) + 1]
        return pair[1] + (battle.navi(side).attack + 1) * pair[2]
    end
    local level = exe5.navi_level(side)
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
-- and holds none of their chips (the custom screen reads every chip's damage, for the hand it may build). A level past
-- the row (which the rules' `validate` says no side has: ../rules/save) reads its last entry, so that a round is set up
-- for every setup a tool asks about.
function navi_level.row(row: { number }): (side: number, chip: Chip) -> number
    return function(side: number, chip: Chip): number
        if exe5.in_party(side) then
            return navi_level.party_chip(side, chip)
        end
        local level = exe5.navi_level(side)
        if level == nil then
            return 0
        end
        return row[math.min(level, #row - 1) + 1]
    end
end

-- The DS's damage of a team navi's own chip in a party (0x0203BF2C): the base of the navi whose chip it is at the
-- side's level and its member's bonus; 999 when no member is that navi (0x0200DEE8 finds none).
function navi_level.party_chip(side: number, chip: Chip): number
    local level = exe5.navi_level(side) :: number
    local count = battle.party(side).count
    for m = 0, count - 1 do
        local s = battle.member(side, m) :: Navi
        local own = s.navi.own_chip
        local data = (s.navi :: any).party
        if own and own.chip == chip and data then
            return data.chip_damage[level + 1] + s.chip_attack_bonus
        end
    end
    return 999
end

return navi_level
