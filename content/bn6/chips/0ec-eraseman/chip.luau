--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): EraseMan's chips keep the pack's records, because the ruleset
-- turns navi chips back by their numbers (AntiNavi, 0xDD to 0x118) and
-- EraseMn[SP]'s damage is a formula. Their records name navi chip subtype
-- 5, which runs this module's `navi_chip`: EraseMan summoned with the
-- record's first parameter as his aim's switching time. When the chips are
-- definitions each is `navi = eraseman.summon { aim_ticks = n }` (20, 16,
-- 12) and this goes.

local eraseman = require("../eraseman/navi")

local by_number = {}

-- `off_802CD5C[5]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return eraseman.summon({ aim_ticks = spec.params[1] })(user, controller, spec)
end

return by_number
