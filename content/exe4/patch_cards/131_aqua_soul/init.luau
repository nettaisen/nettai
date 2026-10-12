--!strict
-- The patch card Aqua Soul (131, アクアソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local aquasoul = require("../navis/megaman/forms/aquasoul").aquasoul

local card: SlottedPatchCard = {
    slot = "0E",
    effects = {
        cards.soul(aquasoul),
        cards.bug("hp", 2),
    },
}

return { ["131-aqua-soul"] = card }
