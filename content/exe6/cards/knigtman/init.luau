--!strict
-- The patch card KnigtMan (36, ナイトマン in the Japanese games), 45 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "knigtman",
    mb = 45,
    effects = {
        cards.hp_percent(20),
        cards.super_armor(true),
        cards.undershirt(true),
        cards.attack_times(2),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}
