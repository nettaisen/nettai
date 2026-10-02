--!strict
-- The patch card CutMan (35, カットマン in the Japanese games), 32 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:cutman",
    mb = 32,
    effects = {
        cards.bug(cards.hp(-100)),
        cards.attack(4),
        cards.charged_shot(charge),
    },
}
