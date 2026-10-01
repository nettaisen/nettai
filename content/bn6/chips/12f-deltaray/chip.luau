--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): DeltaRay keeps the pack's record, because the ruleset turns navi
-- chips back by their numbers (AntiNavi) and the pack's Giga chips are
-- records until the chips' data is content (step 5). Its record names navi
-- chip subtype 28, which runs this module's `navi_chip`: DeltaRay summoned
-- in the palette the record's first parameter picks (`byte_800FD5C`, with
-- ProtoMan's navi palette count 1). When the chip is a definition it is
-- `navi = deltaray.summon { palette = 0 }` and this goes.

local deltaray = require("../deltaray/navi")

local by_number = {}

-- His palette by the record's first parameter (`sub_800FCFC(0xB)`:
-- `byte_80212BB[0xB]`, 1, times `byte_800FD5C[Param1]`).
local PALETTES = { [0] = 0, 0, 0, 2, 3, 0, 0, 0 }

-- `off_802CD5C[28]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local palette = PALETTES[spec.params[1]]
    if palette == nil then
        error(string.format("DeltaRay's palette %d reads past byte_800FD5C", spec.params[1]))
    end
    return deltaray.summon({ palette = palette })(user, controller, spec)
end

return by_number
