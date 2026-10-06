--!strict
-- The patch card KingMan (90, キングマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.custom(2),
        cards.mega_folder(1),
        cards.bug(cards.hp(-150)),
    },
}

return { kingman = card }
