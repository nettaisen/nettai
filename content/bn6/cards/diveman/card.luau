--!strict
-- The patch card DiveMan (104, ダイブマン in the Japanese games), 42 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/diveman",
    name = "DiveMan",
    mb = 42,
    effects = {
        cards.hp(320),
        cards.body("aqua"),
        cards.charged_shot(charge),
        cards.bug(cards.custom(-2)),
        cards.mega_folder(1),
    },
}
