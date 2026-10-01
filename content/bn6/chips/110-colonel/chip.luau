--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): Colonel, Colonel[EX], Colonel[SP] and CrossDiv keep the pack's
-- records, because the ruleset turns navi chips back by their numbers
-- (AntiNavi, 0xDD to 0x118) and the SP chip's damage is a formula;
-- CrossDiv's is a Giga chip nothing names, kept beside them until they go.
-- Their records name navi chip subtype 17, which runs this module's
-- `navi_chip`: Colonel summoned in the palette the record's first parameter
-- picks (`byte_80B8294`: 3 for Colonel[SP]'s 4, else 0), striking
-- CrossDiv's cross when its third is set. When the chips are definitions
-- each is `navi = colonel.summon { palette = n, cross = b }` and this goes.

local colonel = require("../colonel/navi")

local by_number = {}

-- His palette by the record's first parameter (`byte_80B8294`).
local PALETTES = { [0] = 0, 0, 0, 0, 3, 0, 0, 0 }

-- `off_802CD5C[17]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local palette = PALETTES[spec.params[1]]
    if palette == nil then
        error(string.format("Colonel's palette %d reads past byte_80B8294", spec.params[1]))
    end
    return colonel.summon({ palette = palette, cross = spec.params[3] ~= 0 })(user, controller, spec)
end

return by_number
