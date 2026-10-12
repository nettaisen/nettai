--!strict
-- The patch card Wind Soul (123, ウインドソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local windsoul = require("../navis/megaman/forms/windsoul").windsoul

local card: PatchCard = {
    effects = {
        cards.soul(windsoul),
        cards.bug("hp", 2),
    },
}

return { ["123-wind-soul"] = card }
