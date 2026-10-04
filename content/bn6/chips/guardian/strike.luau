--!strict
-- A Guardian statue's strike-back dimming (the original's effect object #0x53, `sub_80E680C`;
-- docs/engine/dimming-chip-effects.md §5): the shared one (@common/guardian/strike), the controller the statue
-- (./statue) starts when an attack breaks it, its telop naming Punisher (`sub_80D4DF0` spawns it with Punisher,
-- bonus 0).

local common = require("@common/guardian/strike")
local punisher = require("../punisher/chip")

return common.make { id = "bn6:guardian/strike", punisher = punisher }
