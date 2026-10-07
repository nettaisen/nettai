--!strict
-- EXE4's light and dark MegaMan (docs/design/exe4-map.md §7). A side's MegaMan is light or dark by his save's
-- light/dark value (NaviStats +0x36, 0x0800F5A6): the rules' setup, which a setup takes from the save (exe4-compat
-- from a recording's). What EXE4 decides by it:
--
-- - **The starting mood** (MegaMan's round setup, 0x0800D872: in a netbattle, battle type 0x46 and on, whatever the
--   mood was; 0x0800F56A): under 470 0 (a dark MegaMan's: his emotion worn out); with the save's Full Synchro at the
--   start (NaviStats +0x1F, a Mod Card's) 0xFF; from 1000 190; else the value / 20 + 128 (a fresh save's 500: 0x99).
--   (Its branch for a value under 500, 64, follows the test under 470 and is never taken.)
-- - **A dark MegaMan clears holy panels** (0x080132E6, from the navi's intake every tick of the fight, unless his
--   NaviStats +0x26 is 2, which no netbattle's is): when the panel he stands on is holy and the value is 499 or less,
--   the panel turns normal. (Its counterpart for a light MegaMan, 0x08013318, turns panel type 11 normal under a
--   value over 499: no stage of EXE4's content has type 11.)

export type Setup = {
    -- The save's light/dark value, its karma (NaviStats +0x36): 0 to 1000; a fresh save's 500 (0x0800D6CA), the
    -- setup's default. (A match's side's `karma`, which nettai-match writes by this field's name.)
    karma: number,
    -- The save's Full Synchro at the start (NaviStats +0x1F: Mod Card 45's), none by default.
    full_synchro_start: boolean,
}

-- 0x0800F56A: the starting mood by the value and the Full Synchro at the start.
local function starting_mood(v: number, full_synchro: boolean): number
    if v < 470 then
        return 0
    elseif full_synchro then
        return 0xFF
    elseif v >= 1000 then
        return 190
    end
    return v // 20 + 128
end

local light_dark = {}

-- A value at or below this is a dark MegaMan's for the holy panels (0x08013306: 0x1F3).
local CLEARS_HOLY = 499

-- 0x0800D872: MegaMan's starting mood.
function light_dark.starting_mood(_side: number): number?
    local setup = rules.setup() :: Setup
    return starting_mood(setup.karma, setup.full_synchro_start)
end

-- 0x080132E6: the holy panel under a dark MegaMan turns normal.
function light_dark.navi_intake(_side: number, navi: Object)
    if (rules.setup() :: Setup).karma > CLEARS_HOLY then
        return
    end
    local x, y = navi.panel_x, navi.panel_y
    local p = field.panel(x, y)
    if p and p.kind == "holy" then
        field.set_type(x, y, "normal")
    end
end

return light_dark
