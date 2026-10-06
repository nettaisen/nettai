--!strict
-- The patch card Serenade (103, セレナード in the Japanese games), 50 MB.

local cards = require("../rules/patch_cards/cards")
local guards = require("../navis/megaman/weapons/guards")

local card: PatchCard = {
    mb = 50,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.charged_shot(guards.reflect),
        cards.chip_recovery(100),
    },
}

return { serenade = card }
