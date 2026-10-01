--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- records of TimeBom1-3 and TimeBom+ (dimming chip subtype 10; TimeBom+
-- is a Program Advance whose recipes name the others) run their chips'
-- `dimming` hooks (chips/timebom), picked by the first chip parameter as
-- the original's bomb picks its row (`byte_80CD8AC`). It goes when the
-- recipes name chips by definition.

local timebom = require("../timebom/chips")

local function dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local row = spec.params[1]
    if row == 0 then
        return timebom.timebom1.dimming(user, spec)
    elseif row == 1 then
        return timebom.timebom.dimming(user, spec)
    end
    -- (Rows 2 to 7 are bombs no chip sets: chips/timebom/countdown.)
    error(string.format("a countdown bomb of row %d has no chip", row))
end

return { dimming_chip = dimming_chip }
