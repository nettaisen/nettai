--!strict
-- The patch card Proto Soul (127, ブルースソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local protosoul = require("../navis/megaman/forms/protosoul").protosoul

local card: PatchCard = {
    effects = {
        cards.soul(protosoul),
        cards.bug("hp", 2),
    },
}

return { ["127-proto-soul"] = card }
