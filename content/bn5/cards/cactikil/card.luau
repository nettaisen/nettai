--!strict
-- The patch card Cactikil (30, サボテコロン in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "cactikil",
    mb = 16,
    effects = {
        cards.hp(60),
        cards.body("wood"),
        cards.super_armor(true),
        cards.bug(cards.float_shoes(false)),
    },
}
