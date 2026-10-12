--!strict
-- EXE4's light and dark MegaMan (docs/design/exe4-map.md §7). A side's MegaMan is light or dark by his save's
-- light/dark value (NaviStats +0x36, 0x0800F5A6): the rules' setup, which a setup takes from the save (exe4-compat
-- from a recording's). What EXE4 decides by it:
--
-- - **The starting mood** (MegaMan's round setup, 0x0800D872: in a netbattle, battle type 0x46 and on, whatever the
--   mood was; 0x0800F56A): under 470 0 (a dark MegaMan's: his emotion worn out); with the save's Full Synchro at the
--   start (NaviStats +0x1F, a patch card's) 0xFF; from 1000 190; else the value / 20 + 128 (a fresh save's 500: 0x99).
--   (Its branch for a value under 500, 64, follows the test under 470 and is never taken.)
-- - **A dark MegaMan clears holy panels** (0x080132E6, from the navi's intake every tick of the fight, unless his
--   NaviStats +0x26 is 2: only a story's other navi, 0x08041100, has it): when the panel he stands on is holy and the
--   value is 499 or less, the panel turns normal.
-- - **A navi that isn't dark closes a hole** (0x08013318, from the intake after it): unless the battle is dimmed, a
--   navi whose value is above 499 standing on a hole (his panel's) mends the panel his body is on (his collision's)
--   to normal. (Its check of 0x0800F49C counts for nothing: the `movs r0, #5` before its `beq` sets the flags it
--   tests. Nor does NaviStats +0x26's 2, which skips it.)
-- - **His palette** (0x0800C08E each tick from the player's update, 0x080EAEEC, and as he is set up, 0x080107DE;
--   0x0800BFFE picks it): a player whose param is set 1; in soul 15 1; in any other soul 0; else 4 in Full Synchro
--   (mood 0xFF), else by the value's tier (0x0800C052: from 1000 4, from 500 0, from 470 3, else 2); then, but in a
--   soul other than 15, 5 more for each step of his color (NaviStats +0x27: the rules' stat `color`, patch cards 59,
--   60, 89 and 90's 1 to 4). Presentation: the sprite's palette. (Another navi's palette is 0x0800B95E's table: EXE4's content has MegaMan
--   alone. No player the engine spawns has a param, and no soul is ported yet.) Tango's light saves hold 1000, its
--   dark ones 460.
-- - **The chips a MegaMan may use** (a chip's `megaman`, its record's +0x15: 1 a light MegaMan's, 2 a dark one's; 94
--   chips state one). The use's preparation ends with the check (0x0800D5D0, in 0x0800D4B4): a light one's chip
--   wants a value of 470 or more, a dark one's less. A chip of the other kind keeps everything it prepared (its
--   chip, its lockout, the Full Synchro it spent, its name on the other console) and only its action becomes the
--   fizzle's (action 0x20, variant 4: the invalid chip's sparkle; rules/chip_use's `refused_keeps_chip`). The check
--   reads no navi number: any navi's use is checked.
-- - **The chips a MegaMan may cut in with** (0x0800D608, which the cut-in asks before it prepares anything,
--   0x08010C64): the same test at 480. So a value of 470 to 479 is a light one's for a use and a dark one's for a
--   cut-in: a light one's dimming chip can't cut in there, though its use works.
--   **Not ported**: at those ten values a dark one's dimming chip passes the cut-in's test and then fails the
--   use's, and the cut-in (0x08010C88) runs the dimming table's entry of the attack's variant whatever the action
--   is: the fizzle's variant 4, FireMan's controller (0x080E31C4), with the refused chip's attack. Here that cut-in
--   starts nothing and the chip stays in the hand (docs/design/exe4-map.md §18 item 65).

local invalid = require("../chips/invalid").invalid
local megaman = require("../navis/megaman").megaman

export type Setup = {
    -- The save's light/dark value, its karma (NaviStats +0x36): 0 to 1000; the default block's and a fresh save's
    -- 500 (0x0800D6BE, 0x0800D6CA), the setup's default. (A match's side's `karma`, which nettai-match writes by this
    -- field's name.)
    karma: number,
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

-- 0x0800D872: MegaMan's starting mood, by the light/dark value and the Full Synchro at the start (NaviStats +0x1F:
-- the rules' stat, patch card 45's, ../patch_cards).
function light_dark.starting_mood(side: number): number?
    local setup = rules.setup() :: Setup
    return starting_mood(setup.karma, battle.navi(side).full_synchro_start)
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

-- A value at or above these is a light MegaMan's: for a chip's use (0x0800D5DE: 0xEB * 2) and for a cut-in
-- (0x0800D618: 0xF0 * 2).
local LIGHT_USES, LIGHT_CUTS_IN = 470, 480

-- Whether the side's MegaMan, light from `light_from`, is the one `chip` asks for (its `megaman`; none: anyone).
local function may_use(chip: Chip?, light_from: number): boolean
    local wants = chip and (chip :: any).megaman
    if wants == nil then
        return true
    end
    local light = (rules.setup() :: Setup).karma >= light_from
    return (wants == "light") == light
end

-- 0x0800D5D0: a chip of the other kind of MegaMan fizzles, its action the invalid chip's.
function light_dark.chip_check(_side: number, _navi: Object, chip: Chip?): Chip?
    if may_use(chip, LIGHT_USES) then
        return nil
    end
    return invalid
end

-- 0x0800D608 (from the cut-in, 0x08010C64): a chip of the other kind of MegaMan doesn't cut in.
function light_dark.cut_in_barred(_side: number, _navi: Object, chip: Chip?): boolean
    return not may_use(chip, LIGHT_CUTS_IN)
end

-- 0x0800C052: the palette by the value's tier.
local function tier(v: number): number
    if v >= 1000 then
        return 4
    elseif v >= 500 then
        return 0
    elseif v >= 470 then
        return 3
    end
    return 2
end

-- 0x0800BFFE: the palette of the side's navi.
function light_dark.navi_palette(side: number, _navi: Object): number?
    local stats = battle.navi(side)
    if stats.navi ~= megaman then
        return nil
    end
    if not stats.form.base then
        -- (A soul's: 0, no color; soul 15's 1 and the color's. No soul is ported yet.)
        return 0
    end
    local palette = if stats.mood == 0xFF then 4 else tier((rules.setup() :: Setup).karma)
    return palette + 5 * stats.color
end

return light_dark
