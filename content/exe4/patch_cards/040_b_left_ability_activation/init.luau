--!strict
-- The patch card B+Left Ability Activation (40, B+左で技発動).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    effects = {
        cards.b_left(chips.anti_chip),
    },
}

return { ["040-b-left-ability-activation"] = card }
