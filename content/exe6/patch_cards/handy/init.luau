--!strict
-- The patch card Handy (54, ハンディース in the Japanese games), 16 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 16,
    effects = {
        cards.hp(100),
        cards.charged_shot(charge),
        cards.bug(cards.chip_recovery(0)),
    },
}

return { handy = card }
