--!strict
-- The patch card CosmoMan (45, コスモマン in the Japanese games), 44 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 44,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.mega_folder(1),
        cards.giga_folder(1),
    },
}

return { cosmoman = card }
