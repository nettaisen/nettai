--!strict
-- The patch card Serenade (103, セレナード in the Japanese games), 50 MB.

local cards = require("../rules/patch-cards/cards")
local guards = require("../navis/megaman/weapons/guards")

return define.patch_card {
    id = "serenade",
    mb = 50,
    effects = {
        cards.chip_recovery(100),
        cards.bug(cards.hp_percent(-20)),
        cards.charged_shot(guards.reflect),
    },
}
