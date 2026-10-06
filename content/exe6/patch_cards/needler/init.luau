--!strict
-- The patch card Needler (66, ニドキャスター in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.super_armor(true),
        cards.attack(1),
        cards.rapid(1),
        cards.charge(1),
        cards.emotion_bug(0),
    },
}

return { needler = card }
