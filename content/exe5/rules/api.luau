--!strict
-- EXE5's API (docs/design/rules-in-luau.md, As built S8, as EXE6's rules/api.luau): what EXE5's chips, navis and
-- libraries ask of EXE5's rules, which keep it in their state and setup. Content outside rules/ asks it here, never
-- the rules' state or setup itself.

local exe5 = {}

-- The side's level (0 to its team navi's story's last), or nil (MegaMan, who takes none): the rules' `level`
-- (rules/save), the count of the save's story flags the init exchange sends. The engine knows nothing of it: the team
-- navis' damage rows read it (lib/navi_level).
function exe5.navi_level(side: number): number?
    local s = rules.setup_of(side) :: { level: number? }?
    local level = if s then s.level else nil
    -- (A side with partners, the DS's party battle, has the DS's level, 0 to 9, 0x021B82B0: its highest where its setup
    -- states none. The party's rules, party-engine's, will say so themselves.)
    if level == nil and battle.party(side).count > 1 then
        return 9
    end
    return level
end

-- The side's light/dark value, MegaMan's karma (NaviStats +0x44): the save's (its setup's `karma`: ./light_dark), or 500
-- in a battle that holds it (battle effect 0x20000, 0x08010EDC at MegaMan's start: nothing reads it before).
local VALUE_HELD = 0x20000

function exe5.light_dark_value(side: number): number
    if bit32.btest(battle.effects(), VALUE_HELD) then
        return 500
    end
    local s = rules.setup_of(side) :: { karma: number }?
    if s == nil then
        error(string.format("side %d plays by no rules: no light/dark value", side))
    end
    return s.karma
end

-- 0x0800DE5C: the palette tier of a light/dark value (MegaMan's palettes, a hit's dark drain: ./light_dark).
function exe5.light_dark_tier(v: number): number
    if v >= 1000 then
        return 4
    elseif v >= 500 then
        return 0
    elseif v >= 470 then
        return 3
    end
    return 2
end

-- The last navi chip used, of either side (0x0203C430): the chip, and the element and the damage word, bonus included,
-- its navi came with; nil before any.
export type LastNaviChip = { chip: Chip, element: number, damage: number }

type Kept = { last_navi_chip: { chip: Chip?, element: number, damage: number } }

-- 0x080E1880's: a navi chip of the block is the last navi chip used, its navi come with `element` and `damage`.
function exe5.keep_navi_chip(chip: Chip?, element: number, damage: number)
    for side = 0, 1 do
        local s = rules.state_of(side) :: Kept?
        if s then
            local last = s.last_navi_chip
            last.chip, last.element, last.damage = chip, element, damage
        end
    end
end

function exe5.last_navi_chip(): LastNaviChip?
    local s = rules.state_of(0) :: Kept?
    if s == nil then
        return nil
    end
    local last = s.last_navi_chip
    local chip = last.chip
    if chip == nil then
        return nil
    end
    return { chip = chip, element = last.element, damage = last.damage }
end

return exe5
