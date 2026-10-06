--!strict
-- The NaviCust supports' dimming controller (the original's effect object #0x79, `sub_80E8FE0`): the shared controller
-- (@exelib/supports/controller, EXE5's too) with EXE6's supports, Rush (./rush), Beat (./beat) and Tango (./tango). The
-- rules spawn it by the role `kinds.support` (rules/roles.luau). See docs/engine/chips.md §2.10.

local beat = require("./beat")
local common = require("@exelib/supports/controller")
local rush = require("./rush")
local tango = require("./tango")

return {
    kind = common.make {
        id = "support/controller",
        rush = rush,
        beat = beat,
        tango = tango,
    },
}
