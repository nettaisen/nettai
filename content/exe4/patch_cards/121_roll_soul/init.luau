--!strict
-- The patch card Roll Soul (121, ロールソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local rollsoul = require("../navis/megaman/forms/rollsoul").rollsoul

local card: SlottedPatchCard = {
    slot = "0E",
    effects = {
        cards.soul(rollsoul),
        cards.bug("hp", 2),
    },
}

return { ["121-roll-soul"] = card }
