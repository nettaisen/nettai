--!strict
-- DrkRecov: the recovery chips' heal (chips/recov/heal.luau) of 1000 HP.
--
-- Its record stays the pack's for now (a dark chip, with Recov10 as its
-- substitute): this is its action, which registration by number runs
-- (chips/09a-recov10, docs/design/content-model-v2.md §12).

local heal = require("../recov/heal")

return {
    action = heal.action { id = "drkrecov/action", hp = 1000 },
}
