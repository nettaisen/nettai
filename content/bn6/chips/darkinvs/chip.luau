--!strict
-- DarkInvs, a dark chip: its navi runs the Cross special, fighting on its
-- own with chips by its HP (the navi's cross-special request;
-- lib/navi-boost/controller).
--
-- Its record stays the pack's for now (it has a substitute chip, which a
-- definition can't give yet): this is its `dimming` hook, which
-- registration by number runs (chips/121-darkinvs).

local navi_boost = require("../../lib/navi-boost/controller")

return { dimming = navi_boost.dark }
