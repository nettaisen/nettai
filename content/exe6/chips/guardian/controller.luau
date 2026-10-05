--!strict
-- The Guardian chip's dimming controller (the original's effect object
-- #0x52, `sub_80E6758`; docs/engine/dimming-chip-effects.md §5): the usual
-- dimming phases, with an effect that places a Guardian statue (./statue)
-- on the panel in front of the user if it is free solid ground, striking
-- with the damage plus the bonus, and waits 30 ticks. The behavior is EXE5's
-- too (@exelib/guardian/controller), made here with EXE6's statue.

local common = require("@exelib/guardian/controller")
local statue = require("./statue")

return common.make { id = "guardian/controller", statue = statue.spawn }
