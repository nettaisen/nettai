--!strict
-- The patch card Alpha (104, プロト in the Japanese games), 55 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 55,
    effects = {
        cards.hp(100),
        cards.bug(cards.custom(-3)),
        cards.giga_folder(2),
    },
}

return { alpha = card }
