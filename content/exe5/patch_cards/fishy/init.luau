--!strict
-- The patch card Fishy (5, キオルシン in the Japanese games), 18 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 18,
    effects = {
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.bug(cards.step_bug(1)),
    },
}

return { fishy = card }
