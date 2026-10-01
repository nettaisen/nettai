--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): ElmntMan, ElmntMn[EX] and ElmntMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 16, which runs this module's `navi_chip`: ElmntMan
-- summoned with the record's first parameter as the ticks each element
-- shows. When the chips are definitions each is `navi = elmntman.summon {
-- cycle_ticks = n }` and this goes.

local elmntman = require("../elmntman/navi")

local by_number = {}

-- `off_802CD5C[16]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return elmntman.summon({ cycle_ticks = spec.params[1] })(user, controller, spec)
end

return by_number
