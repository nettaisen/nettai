--!strict
-- The patch card Draggin (73, ドラグリン in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 11,
    effects = {
        cards.hp_percent(5),
        cards.body("fire"),
        cards.bug(cards.status_bug(1)),
    },
}

return { draggin = card }
