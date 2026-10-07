--!strict
-- EXE4's light and dark MegaMan, as far as ported (docs/design/exe4-map.md §18 item 12): the save's light/dark value
-- (NaviStats +0x36, 0x0800F5A6; Tango's light saves hold 1000, its dark ones 460), by which a navi that isn't dark
-- closes the hole he stands on, and by which MegaMan is drawn:
--
-- - **His palette** (0x0800C08E each tick from the player's update, 0x080EAEEC, and as he is set up, 0x080107DE;
--   0x0800BFFE picks it): a player whose param is set 1; in soul 15 1; in any other soul 0; else 4 in Full Synchro
--   (mood 0xFF), else by the value's tier (0x0800C052: from 1000 4, from 500 0, from 470 3, else 2); then 5 more for
--   each step of his color (NaviStats +0x27, a Mod Card's, which no stats here carry yet: exe4-compat refuses one).
--   Presentation: the sprite's palette. (Another navi's palette is 0x0800B95E's table: EXE4's content has MegaMan
--   alone. No player the engine spawns has a param, and no soul is ported yet.)
--
-- Not yet (exe4-map.md §18): what else reads the value (the mood, 0x0800D836; the dark chips).

local megaman = require("../navis/megaman").megaman

export type Setup = {
    -- The save's light/dark value, its karma (NaviStats +0x36): 0 to 1000; the default block's 500 (0x0800D6BE),
    -- the setup's default. (A match's side's `karma`, which nettai-match writes by this field's name.)
    karma: number,
}

-- The value above which a navi closes a hole (0x08013348: `cmp r0, 0x1F3; ble`): a light one's, and the default
-- 500.
local CLOSES_ABOVE = 499

-- 0x08013318, each tick of the navi's intake: unless the battle is dimmed, a navi whose value is above 499 standing
-- on a hole (his panel's) mends the panel his body is on (his collision's) to normal. (Its check of 0x0800F49C counts
-- for nothing: the `movs r0, #5` before its `beq` sets the flags it tests. Nor does NaviStats +0x26's 2, which skips
-- it: only a story's other navi, 0x08041100, has it.)
local function navi_intake(_side: number, navi: Object)
    if battle.dimmed() then
        return
    end
    local p = field.panel(navi.panel_x, navi.panel_y)
    if p == nil or p.kind ~= "hole" then
        return
    end
    if (rules.setup() :: Setup).karma <= CLOSES_ABOVE then
        return
    end
    field.set_type(navi.collision.panel_x, navi.collision.panel_y, "normal")
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
local function navi_palette(side: number, _navi: Object): number?
    local stats = battle.navi(side)
    if stats.navi ~= megaman then
        return nil
    end
    if not stats.form.base then
        -- (A soul's: 0; soul 15's 1. No soul is ported yet.)
        return 0
    end
    if stats.mood == 0xFF then
        return 4
    end
    return tier((rules.setup() :: Setup).karma)
end

return {
    navi_intake = navi_intake,
    navi_palette = navi_palette,
}
