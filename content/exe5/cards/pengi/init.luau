--!strict
-- The patch card Pengi (69, コリペン in the Japanese games), 18 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "pengi",
    mb = 18,
    effects = {
        cards.hp(150),
        cards.body("aqua"),
        cards.bug(cards.panel_trail("ice", 3)),
    },
}
