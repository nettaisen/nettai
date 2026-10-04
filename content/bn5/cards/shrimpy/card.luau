--!strict
-- The patch card Shrimpy (63, エビロン in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")
local spreads = require("../../navis/megaman/weapons/spreads")

return define.patch_card {
    id = "shrimpy",
    mb = 10,
    effects = {
        cards.body("aqua"),
        cards.charged_shot(spreads.bubble),
        cards.bug(cards.attack(-1)),
    },
}
