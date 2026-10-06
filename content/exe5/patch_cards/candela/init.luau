--!strict
-- The patch card Candela (9, キャンデービル in the Japanese games), 9 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 9,
    effects = {
        cards.body("fire"),
        cards.auto_heal(true),
        cards.bug(cards.back_special(nil)),
    },
}

return { candela = card }
