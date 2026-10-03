--!strict
-- Anubis's and PoisPhar's dimming controller (the original's effect object
-- #0x33, `sub_80E40EC`; docs/engine/dimming-chip-effects.md §7): the usual
-- dimming phases (lib/dimming), with an effect that drops the chip's
-- statue (./statue) on the panel in front of the user, whatever is there,
-- and waits 120 ticks. The behavior is BN5's too
-- (@common/anubis/controller), made here with BN6's statue.

local common = require("@common/anubis/controller")
local statue = require("./statue")

return common.make { id = "bn6:anubis/controller", statue = statue.spawn }
