--!strict
-- The NaviCust supports' dimming controller (EXE5's effect object #0x74, 0x080E8F50; EXE6's #0x79): the shared controller
-- (@exelib/supports/controller, EXE6's code) with EXE5's supports, Rush (./rush), Beat (./beat) and Tango (./tango). The
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
