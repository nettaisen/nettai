--!strict
-- The patch card Count (113, ハクシャク in the Japanese games), 60 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 60,
    effects = {
        cards.charged_shot(charge),
        cards.mega_folder(1),
        cards.bug(cards.status_bug(2)),
    },
}

return { count = card }
