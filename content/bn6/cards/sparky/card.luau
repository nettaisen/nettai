--!strict
-- The patch card Sparky (60, ユラ in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn6:sparky",
    mb = 8,
    effects = {
        cards.bug(cards.hp_percent(-5)),
        cards.body("elec"),
        cards.float_shoes(true),
        cards.step_bug(0),
        cards.status_bug(0),
    },
}
