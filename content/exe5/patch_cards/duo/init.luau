--!strict
-- The patch card Duo (106, デューオ in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.hp_percent(40),
        cards.mega_folder(1),
        cards.giga_folder(1),
    },
}

return { duo = card }
