--!strict
-- The patch card Wood Soul (132, ウッドソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local woodsoul = require("../navis/megaman/forms/woodsoul").woodsoul

local card: SlottedPatchCard = {
    slot = "0E",
    effects = {
        cards.soul(woodsoul),
        cards.bug("hp", 2),
    },
}

return { ["132-wood-soul"] = card }
