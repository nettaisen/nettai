--!strict
-- The patch card Tora's Tactics (100, トラキチの 戦じゅつ in the Japanese games), 64 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "toras-tactics",
    mb = 64,
    effects = {
        cards.rush(true),
        cards.beat(true),
        cards.tango(true),
    },
}
