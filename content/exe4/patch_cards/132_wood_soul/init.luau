--!strict
-- The patch card Wood Soul (132, ウッドソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local woodsoul = require("../navis/megaman/forms/woodsoul").woodsoul

local card: PatchCard = {
    effects = {
        cards.soul(woodsoul),
        cards.bug("hp", 2),
    },
}

return { ["132-wood-soul"] = card }
