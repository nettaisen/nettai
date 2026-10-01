--!strict
-- FireBrn1, FireBrn2 and FireBrn3: the burner's flame (lib/burner) on the
-- panel ahead, burning 30 ticks and spreading two panels forward, cracking
-- the panels it leaves. The three differ only in their damage.
--
-- Their records stay the pack's for now (WideBrn's Program Advance
-- recipes name them by number, as does the Cross special): these are
-- their actions, which registration by number runs (chips/014-firebrn1,
-- docs/design/content-model-v2.md §12).

local burn = require("../../lib/burner/burn")

local function fire_burn(id: string): Action
    return burn.action {
        id = id,
        burner = burn.burners.fire,
        flame = { ticks = 30, spread = 2, cracks = true },
    }
end

return {
    { action = fire_burn("firebrn1/action") },
    { action = fire_burn("firebrn2/action") },
    { action = fire_burn("firebrn3/action") },
}
