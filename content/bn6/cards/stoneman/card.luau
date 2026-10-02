--!strict
-- The patch card StoneMan (84, ストーンマン in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards")

return define.patch_card {
    id = "stoneman",
    name = "StoneMan",
    mb = 45,
    effects = {
        cards.hp(360),
        cards.super_armor(true),
        cards.bug(cards.panel_trail("crack", 1)),
    },
}
