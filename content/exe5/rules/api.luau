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
    return if s then s.level else nil
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
