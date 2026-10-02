--!strict
-- The patch card Count (113, ハクシャク in the Japanese games), 60 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/count",
    name = "Count",
    mb = 60,
    effects = {
        cards.charged_shot(charge),
        cards.mega_folder(1),
        cards.bug(cards.status_bug(2)),
    },
}
