--!strict
-- AntiNavi: a hidden trap that turns the other side's next navi chip
-- around, so its navi comes for this side instead (the dimming service's
-- `sub_800BDB2`). Its controller is the trap chips' (lib/traps/controller);
-- it sets no object of its own.
--
-- Its record stays the pack's for now (the dimming service names it by
-- number, and BodyGrd's recipe names it): this is its `dimming` hook, which
-- registration by number runs (chips/0ba-antinavi).

local traps = require("../../lib/traps/controller")

return { dimming = traps.hook() }
