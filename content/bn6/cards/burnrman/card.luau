--!strict
-- The patch card BurnrMan (44, バーナーマン in the Japanese games), 29 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn6:burnrman",
    mb = 29,
    effects = {
        cards.hp(120),
        cards.body("fire"),
        cards.attack(2),
        cards.bug(cards.custom_damage(40)),
        cards.bug(cards.emotion_bug(1)),
    },
}
