--!strict
-- The patch card Guts Soul (122, ガッツソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local gutssoul = require("../navis/megaman/forms/gutssoul").gutssoul

local card: SlottedPatchCard = {
    slot = "0E",
    effects = {
        cards.soul(gutssoul),
        cards.bug("hp", 2),
    },
}

return { ["122-guts-soul"] = card }
