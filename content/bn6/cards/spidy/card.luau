--!strict
-- The patch card Spidy (16, クーモス in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn6:spidy",
    mb = 8,
    effects = {
        cards.hp(60),
        cards.body("wood"),
        cards.bug(cards.panel_trail("grass", 2)),
    },
}
