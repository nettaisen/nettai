--!strict
-- The NaviCust supports' dimming controller (BN5's effect object #0x74, 0x080E8F50; BN6's #0x79): the shared controller
-- (@common/supports/controller, BN6's code) with BN5's supports, Rush (./rush), Beat (./beat) and Tango (./tango). The
-- ruleset spawns it by the role `kinds.support` (rules/roles.luau). See docs/engine/chips.md §2.10.

local beat = require("./beat")
local common = require("@common/supports/controller")
local rush = require("./rush")
local tango = require("./tango")

return {
    kind = common.make {
        id = "bn5:support/controller",
        rush = rush,
        beat = beat,
        tango = tango,
    },
}
