--!strict
-- The patch card Anaconda (10, ダイジャン in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "anaconda",
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("wood"),
        cards.bug(cards.rush(false)),
        cards.bug(cards.beat(false)),
        cards.bug(cards.tango(false)),
    },
}
