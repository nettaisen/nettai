--!strict
-- The patch card Walla (68, ウォーラ in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "walla",
    mb = 12,
    effects = {
        cards.hp(60),
        cards.bug(cards.first_barrier(nil)),
        cards.custom(1),
    },
}
