--!strict
-- The patch card Melody (25, プルメロ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.hp(90),
        cards.bug(cards.super_armor(false)),
        cards.step_bug(0),
    },
}

return { melody = card }
