--!strict
-- The patch card Eleogre (20, エレオーガ in the Japanese games), 16 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 16,
    effects = {
        cards.hp(50),
        cards.body("elec"),
        cards.emotion_bug(0),
        cards.status_bug(0),
    },
}

return { eleogre = card }
