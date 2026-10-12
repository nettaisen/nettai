--!strict
-- The patch card Metal Soul (129, メタルソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local metalsoul = require("../navis/megaman/forms/metalsoul").metalsoul

local card: SlottedPatchCard = {
    slot = cards.SLOT_0E,
    effects = {
        cards.soul(metalsoul),
        cards.bug("hp", 2),
    },
}

return { ["129-metal-soul"] = card }
