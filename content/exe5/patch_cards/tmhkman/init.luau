--!strict
-- The patch card TmhkMan (95, トマホークマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(200),
        cards.body("wood"),
        cards.attack(3),
        cards.bug(cards.mega_folder(-1)),
        cards.super_armor(true),
    },
}

return { tmhkman = card }
