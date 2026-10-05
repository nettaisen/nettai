--!strict
-- The patch card Count (113, ハクシャク in the Japanese games), 60 MB.

local cards = require("../rules/patch-cards/cards")
local charge = require("@self/charge")

return define.patch_card {
    id = "count",
    mb = 60,
    effects = {
        cards.charged_shot(charge),
        cards.mega_folder(1),
        cards.bug(cards.status_bug(2)),
    },
}
