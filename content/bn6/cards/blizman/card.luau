--!strict
-- The patch card BlizMan (46, ブリザードマン in the Japanese games), 30 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:blizman",
    mb = 30,
    effects = {
        cards.hp(60),
        cards.body("aqua"),
        cards.bug(cards.float_shoes(false)),
        cards.charged_shot(charge),
    },
}
