--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- records of Anubis and PoisPhar (dimming chip subtype 17; PoisPhar's
-- recipe names Anubis) run their chips' `dimming` hooks (chips/anubis,
-- chips/poisphar), picked by the first chip parameter as the original's
-- statue picks its row (`byte_80CF0C0`). It goes when the recipes name
-- chips by definition.

local anubis = require("../anubis/chip")
local poisphar = require("../poisphar/chip")

local function dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local row = spec.params[1]
    if row == 0 then
        return anubis.dimming(user, spec)
    elseif row == 1 then
        return poisphar.dimming(user, spec)
    end
    error(string.format("an Anubis statue of row %d reads past byte_80CF0C0", row))
end

return { dimming_chip = dimming_chip }
