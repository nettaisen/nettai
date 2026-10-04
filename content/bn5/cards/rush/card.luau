--!strict
-- The patch card Rush (60, ラッシュ in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "rush",
    mb = 11,
    effects = {
        cards.rush(true),
        cards.bug(cards.beat(false)),
        cards.bug(cards.tango(false)),
    },
}
