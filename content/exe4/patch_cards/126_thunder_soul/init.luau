--!strict
-- The patch card Thunder Soul (126, サンダーソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local thundersoul = require("../navis/megaman/forms/thundersoul").thundersoul

local card: PatchCard = {
    effects = {
        cards.soul(thundersoul),
        cards.bug("hp", 2),
    },
}

return { ["126-thunder-soul"] = card }
