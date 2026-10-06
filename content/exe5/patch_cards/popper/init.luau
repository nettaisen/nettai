--!strict
-- The patch card Popper (57, ミノゴロモン in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.body("wood"),
        cards.chip_recovery(30),
        cards.bug(cards.panel_trail("grass", 1)),
    },
}

return { popper = card }
