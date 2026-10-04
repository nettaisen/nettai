--!strict
-- The patch card Popper (57, ミノゴロモン in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "popper",
    mb = 20,
    effects = {
        cards.body("wood"),
        cards.chip_recovery(30),
        cards.bug(cards.panel_trail("grass", 1)),
    },
}
