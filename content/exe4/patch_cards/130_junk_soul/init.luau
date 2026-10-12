--!strict
-- The patch card Junk Soul (130, ジャンクソウル).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local junksoul = require("../navis/megaman/forms/junksoul").junksoul

local card: PatchCard = {
    effects = {
        cards.soul(junksoul),
        cards.bug("hp", 2),
    },
}

return { ["130-junk-soul"] = card }
