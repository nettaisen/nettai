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
--   NaviStats +0x26 is 2: only a story's other navi, 0x08041100, has it): when the panel he stands on is holy and the
--   value is 499 or less, the panel turns normal.
-- - **A navi that isn't dark closes a hole** (0x08013318, from the intake after it): unless the battle is dimmed, a
--   navi whose value is above 499 standing on a hole (his panel's) mends the panel his body is on (his collision's)
--   to normal. (Its check of 0x0800F49C counts for nothing: the `movs r0, #5` before its `beq` sets the flags it
--   tests. Nor does NaviStats +0x26's 2, which skips it.)

export type Setup = {
    -- The save's light/dark value, its karma (NaviStats +0x36): 0 to 1000; the default block's and a fresh save's
    -- 500 (0x0800D6BE, 0x0800D6CA), the setup's default. (A match's side's `karma`, which nettai-match writes by this field's name.)
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

-- A value at or below this is a dark MegaMan's (0x08013306 and 0x08013348: 0x1F3); above it a navi closes a hole.
local DARK_AT_MOST = 499

-- 0x0800D872: MegaMan's starting mood.
function light_dark.starting_mood(_side: number): number?
    local setup = rules.setup() :: Setup
    return starting_mood(setup.karma, setup.full_synchro_start)
end

-- Each tick of the navi's intake: 0x080132E6, the holy panel under a dark MegaMan turns normal; 0x08013318, a navi
-- that isn't dark closes the hole he stands on.
function light_dark.navi_intake(_side: number, navi: Object)
    local dark = (rules.setup() :: Setup).karma <= DARK_AT_MOST
    local x, y = navi.panel_x, navi.panel_y
    local p = field.panel(x, y)
    if dark and p and p.kind == "holy" then
        field.set_type(x, y, "normal")
    end
    if battle.dimmed() or dark then
        return
    end
    p = field.panel(x, y)
    if p and p.kind == "hole" then
        field.set_type(navi.collision.panel_x, navi.collision.panel_y, "normal")
    end
end

return light_dark
