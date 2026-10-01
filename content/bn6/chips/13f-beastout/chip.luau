--!strict
-- The BeastOut chip used from the hand: instant chip effect 0
-- (`sub_80104E0`, `off_80EC3F0[0]`), run once by action 0x1C. A sparkle
-- rises by the navi, as for a plus chip (the two share the code).

local plus = require("../../lib/instant/plus")

local beast_out = {}

-- `sub_80104E0`.
function beast_out.instant_chip(user: Object, _spec: InstantChipSpec)
    plus.sparkle(user)
end

return beast_out
