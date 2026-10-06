--!strict
-- The patch card NumbrMan (31, ナンバーマン in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 35,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.custom(3),
        cards.mega_folder(1),
        cards.giga_folder(1),
    },
}

return { numbrman = card }
