--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): JudgeMan, JudgeMn[EX] and JudgeMn[SP] keep the pack's records,
-- because the ruleset turns navi chips back by their numbers (AntiNavi,
-- 0xDD to 0x118) and the SP chip's damage is a formula. Their records name
-- navi chip subtype 15, which runs this module's `navi_chip`: JudgeMan
-- summoned with the record's first parameter as `book_damage`. When the
-- chips are definitions each is
-- `navi = judgeman.summon { book_damage = n }` (20, 30, 40) and this goes.

local judgeman = require("../judgeman/navi")

local by_number = {}

-- `off_802CD5C[15]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    return judgeman.summon({ book_damage = spec.params[1] })(user, controller, spec)
end

return by_number
