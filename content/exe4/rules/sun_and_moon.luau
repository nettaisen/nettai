--!strict
-- RedSun's and BlueMoon's damage (EXE4's damage formulas 47 and 48, 0x080195CE and 0x08019606): a base (RedSun's 100,
-- BlueMoon's 200) and 40 more for each ten of the side's wins over losses against the other version, up to 100 (none
-- under ten): the side's setup's `version_wins` (the battle's copy, 0x0203BE72 + 8 a side, which the link records
-- bring; the status screen's "VS BM Battles"). Under event flag 0x1187 (0x080406A4), which no netbattle sets (./flow),
-- the base and 200 whatever the wins.

local sun_and_moon = {}

-- A Giga chip's damage on side `side` (a chip's `damage`, a function of the side), from `base`.
function sun_and_moon.damage(base: number): (side: number, chip: Chip) -> number
    return function(side: number, _chip: Chip): number
        local wins = rules.setup_of(side).version_wins :: number
        if wins <= 9 then
            return base
        end
        return base + (math.min(wins, 100) // 10) * 40
    end
end

return sun_and_moon
