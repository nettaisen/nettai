--!strict
-- The patch card Drain (59, チクリート in the Japanese games), 18 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "drain",
    mb = 18,
    effects = {
        cards.chip_recovery(20),
        cards.hp_drain(0),
        cards.bug(cards.hp(-80)),
    },
}
