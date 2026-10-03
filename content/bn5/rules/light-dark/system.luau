--!strict
-- BN5's light and dark MegaMan (docs/design/bn5-map.md §6.1). A side's MegaMan is light or dark by his save's
-- light/dark value (NaviStats +0x44, read through 0x08012882): the system's setup, which a setup takes from the
-- save (bn5-compat from a recording's). Two of BN5's rules read it:
--
-- - **A dark MegaMan clears holy panels** (0x08017136, from the navi's intake every tick of the fight): when the
--   panel he stands on is holy and the value is 499 or less, the panel turns Normal. Any holy panel, whoever made
--   it.
-- - **The chips a MegaMan may use** (0x08010118, at the end of a chip use's preparation, 0x080100E6): a chip's
--   `megaman` field (its record's +0x15) asks for a light MegaMan (a value of 470 or more) or a dark one (less);
--   any navi but MegaMan (NaviStats +0x29 not 0) may use it. A chip he may not use becomes the invalid chip
--   (0x080100EC: chip 0x185, action 0x1A's variant 3 with no parameters), which shows a sparkle.
--
-- (The dark chips' own refusals and costs, 0x08010030, wait for BN5's dark chip rules: §6.)

local invalid = require("../../chips/invalid/chip")
local megaman = require("../../navis/megaman/navi")

export type Setup = {
    -- The save's light/dark value (NaviStats +0x44).
    value: number,
}

-- A value at or below this is a dark MegaMan's for the holy panels (0x08017136: 0x1F3).
local CLEARS_HOLY = 499
-- A value at or above this is a light MegaMan's for the chips (0x08010118: 0xEB * 2).
local LIGHT_CHIPS = 470

return define.system {
    id = "bn5:light-dark",
    setup = { value = "u16" },
    hooks = {
        -- 0x08017136: the holy panel under a dark MegaMan turns Normal.
        navi_intake = function(_side: number, navi: Object)
            if (system.setup() :: Setup).value > CLEARS_HOLY then
                return
            end
            local x, y = navi.panel_x, navi.panel_y
            local p = field.panel(x, y)
            if p and p.kind == "holy" then
                field.set_type(x, y, "normal")
            end
        end,
        -- 0x08010118: a chip for the other kind of MegaMan becomes the invalid chip.
        chip_check = function(side: number, _navi: Object, chip: Chip?): Chip?
            if chip == nil or battle.navi(side).navi ~= megaman then
                return nil
            end
            local wants = (chip :: any).megaman
            if wants == nil then
                return nil
            end
            local light = (system.setup() :: Setup).value >= LIGHT_CHIPS
            if (wants == "light") == light then
                return nil
            end
            return invalid
        end,
    },
}
