--!strict
-- The patch card BigHat (24, グラサン in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "bighat",
    name = "BigHat",
    mb = 8,
    effects = {
        cards.bug(cards.super_armor(false)),
        cards.charged_shot(charge),
    },
}
