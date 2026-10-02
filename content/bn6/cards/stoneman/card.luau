--!strict
-- The patch card StoneMan (84, ストーンマン in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/stoneman",
    name = "StoneMan",
    mb = 45,
    effects = {
        cards.hp(360),
        cards.super_armor(true),
        cards.bug(cards.panel_trail("crack", 1)),
    },
}
