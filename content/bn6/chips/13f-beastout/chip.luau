--!strict
-- Registration by number (docs/design/content-model-v2.md §12): the
-- BeastOut chip used from the hand and the invalid chip (the chip an
-- illegal selection becomes) keep the pack's records, because the custom
-- screen names them by number (0x13F, 0x185). Their records name instant
-- chip effect 0 (`sub_80104E0`, `off_80EC3F0[0]`), which runs this
-- module's `instant_chip`: a sparkle rises by the navi, as for a plus chip
-- (the two share the code, lib/instant/plus). It goes when the custom
-- screen names its chips by role (`roles.chips.beast_out`, `invalid`:
-- phase C, step 10).

local plus = require("../../lib/instant/plus")

local beast_out = {}

-- `sub_80104E0`.
function beast_out.instant_chip(user: Object, _spec: InstantChipSpec)
    plus.sparkle(user)
end

return beast_out
