--!strict
-- The BeastOut chip used from the hand: instant chip effect 0
-- (`sub_80104E0`, `off_80EC3F0[0]`), run once by action 0x1C. A sparkle
-- rises by the navi, as for a plus chip (the two share the code).

local plus_chip = require("../0c0-atk-10/chip")

local beast_out = {}

-- `sub_80104E0`.
function beast_out.instant_chip(user: Object, _spec: InstantChipSpec)
    plus_chip.sparkle(user)
end

return beast_out
