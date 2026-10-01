--!strict
-- NumbrBl: a cut-in chip whose NumberMan sets four balls rolling down the
-- user's row; its damage is the last two digits of the user's HP
-- (docs/engine/dimming-chip-effects.md §15).
--
-- Its record stays the pack's for now (its damage is formula 21, which a
-- definition can't give yet): this is its `dimming` hook, which
-- registration by number runs (chips/08a-numbrbl).

local controller = require("./controller")

return { dimming = controller.hook(4) }
