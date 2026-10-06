--!strict
-- The patch card N.O (72, N.O in the Japanese games), 13 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 13,
    effects = {
        cards.hp(100),
        cards.auto_heal(true),
        cards.hp_drain(0),
        cards.bug(cards.status_bug(1)),
    },
}

return { ["n-o"] = card }
