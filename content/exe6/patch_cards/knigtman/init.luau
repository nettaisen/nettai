--!strict
-- The patch card KnigtMan (36, ナイトマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.hp_percent(20),
        cards.attack_times(2),
        cards.super_armor(true),
        cards.undershirt(true),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}

return { knigtman = card }
