--!strict
-- The patch card Ratty (7, チュートン in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "ratty",
    mb = 10,
    effects = {
        cards.gauge("fast"),
        cards.bug(cards.hp_percent(-20)),
    },
}
