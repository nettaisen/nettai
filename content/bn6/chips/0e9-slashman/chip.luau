--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): SlashMan, SlashMn[EX] and SlashMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 4, which runs this module's `navi_chip`: SlashMan
-- summoned with the record's first parameter as his waves' damage. When the
-- chips are definitions each is `navi = slashman.summon { wave_damage = n
-- }` and this goes.

local slashman = require("../slashman/navi")

local by_number = {}

-- `off_802CD5C[4]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return slashman.summon({ wave_damage = spec.params[1] })(user, controller, spec)
end

return by_number
