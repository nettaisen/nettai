--!strict
-- WideBrn1, WideBrn2 and WideBrn3 (FireBrn's Program Advances): the
-- burner's flame (lib/burner) on the panel ahead, burning 60 ticks and
-- spreading two panels forward on three rows, cracking the panels it
-- leaves. The three differ only in their damage.
--
-- Their records stay the pack's for now (Program Advances): these are
-- their actions, which registration by number runs (chips/014-firebrn1,
-- docs/design/content-model-v2.md §12).

local burn = require("../../lib/burner/burn")

local function wide_burn(id: string): Action
    return burn.action {
        id = id,
        burner = burn.burners.fire,
        flame = { ticks = 60, spread = 2, wide = true, cracks = true },
    }
end

return {
    { action = wide_burn("widebrn1/action") },
    { action = wide_burn("widebrn2/action") },
    { action = wide_burn("widebrn3/action") },
}
