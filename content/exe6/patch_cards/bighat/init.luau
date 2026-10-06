--!strict
-- The patch card BigHat (24, グラサン in the Japanese games), 8 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 8,
    effects = {
        cards.bug(cards.super_armor(false)),
        cards.charged_shot(charge),
    },
}

return { bighat = card }
