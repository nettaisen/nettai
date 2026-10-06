--!strict
-- The patch card ThunMan (90, サンダーマン in the Japanese games), 36 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 36,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.chip_recovery(10),
        cards.bug(cards.custom(-1)),
        cards.bug(cards.hp_drain(1)),
    },
}

return { thunman = card }
