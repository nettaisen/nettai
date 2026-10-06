--!strict
-- The patch card DiveMan (104, ダイブマン in the Japanese games), 42 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 42,
    effects = {
        cards.hp(320),
        cards.body("aqua"),
        cards.bug(cards.custom(-2)),
        cards.mega_folder(1),
        cards.charged_shot(charge),
    },
}

return { diveman = card }
