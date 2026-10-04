--!strict
-- The patch card GutsMan (32, ガッツマン in the Japanese games), 35 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "gutsman",
    mb = 35,
    effects = {
        cards.hp(300),
        cards.undershirt(true),
        cards.bug(cards.first_barrier(nil)),
    },
}
