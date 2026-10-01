--!strict
-- Registration by number (docs/design/content-model-v2.md §5.5, "As
-- built"): Roll, Roll2 and Roll3 keep the pack's records, because the
-- ruleset turns navi chips back by their numbers (AntiNavi, 0xDD to 0x118).
-- Their records name navi chip subtype 0, which runs this module's
-- `navi_chip`: Roll summoned with what the record's first parameter picks
-- (5: three rounds of her whip, which no chip has; 4, Roll3's: her second
-- palette). When the chips are definitions each is `navi = roll.summon {
-- rounds = 1, palette = n }` and this goes.

local roll = require("../roll/navi")

local by_number = {}

-- `off_802CD5C[0]`.
function by_number.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local level = spec.params[1]
    local summon = roll.summon { rounds = if level == 5 then 3 else 1, palette = if level == 4 then 1 else 0 }
    return summon(user, controller, spec)
end

return by_number
