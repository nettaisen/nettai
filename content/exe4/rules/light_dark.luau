--!strict
-- EXE4's light and dark MegaMan: a side's MegaMan is light or dark by his light/dark value (NaviStats +0x36: the
-- rules' setup's `karma`, a save's, which the PET keeps in the block a netbattle sends; Tango's light saves hold 1000,
-- its dark ones 460). What EXE4's battle reads of it so far is his palette:
--
-- - **His palette** (0x0800C08E each tick from the player's update, 0x080EAEEC; 0x0800BFFE picks it): a player whose
--   param is set 1; in soul 15 1; in any other soul 0; else 4 in Full Synchro (mood 0xFF), else by the value's tier
--   (0x0800C052: from 1000 4, from 500 0, from 470 3, else 2); then 5 more for each step of his color (NaviStats
--   +0x27, a Mod Card's, which no stats here carry yet: exe4-compat refuses one). Presentation: the sprite's palette.
--
-- (Another navi's palette is 0x0800B95E's table: EXE4's content has MegaMan alone. No player the engine spawns has a
-- param, and no soul is ported yet: `docs/design/exe4-map.md` §18.)

local megaman = require("../navis/megaman").megaman

local light_dark = {}

-- The light/dark value's setup field, and Tango's light saves' value (the setup's default).
export type Setup = {
    karma: number,
}
light_dark.LIGHT = 1000

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
        -- (A soul's: 0; soul 15's 1. No soul is ported yet.)
        return 0
    end
    if stats.mood == 0xFF then
        return 4
    end
    return tier((rules.setup() :: Setup).karma)
end

return light_dark
