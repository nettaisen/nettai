--!strict
-- ColorPt's and DblPoint's dimming controller (the original's effect
-- object #0x50, `sub_80E6480`; docs/engine/dimming-chips.md §10.1). The
-- usual dimming phases (lib/dimming), with an effect (`sub_80E64C4`) that
-- sends a point (point.luau) from each panel of the user's front column
-- that is solid, free and may go to the other side (`sub_80E650A`), then
-- waits until they are all gone. The behavior is BN5's too
-- (@exelib/colorpt/controller), made here with BN6's point.

local common = require("@exelib/colorpt/controller")
local point = require("./point")

return common.make { id = "colorpt/controller", point = point.spawn }
