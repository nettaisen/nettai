--!strict
-- The patch card SearchMn (99, サーチマン in the Japanese games), 37 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "searchmn",
    name = "SearchMn",
    mb = 37,
    effects = {
        cards.hp_percent(5),
        cards.bug(cards.air_shoes(false)),
        cards.charged_shot(charge),
        cards.chip_shuffle(true),
    },
}
