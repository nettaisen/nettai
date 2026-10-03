--!strict
-- The patch card Handy (54, ハンディース in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:handy",
    mb = 16,
    effects = {
        cards.hp(100),
        cards.charged_shot(charge),
        cards.bug(cards.chip_recovery(0)),
    },
}
