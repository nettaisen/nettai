--!strict
-- The patch card Maddy's Custom (98, まどいの カスタマイズ in the Japanese games), 5 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "maddys-custom",
    mb = 5,
    effects = {
        cards.bug(cards.emotion_bug(1)),
        cards.bug(cards.status_bug(2)),
    },
}
