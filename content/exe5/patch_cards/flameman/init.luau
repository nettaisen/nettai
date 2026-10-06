--!strict
-- The patch card FlameMan (38, フレイムマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.hp_percent(10),
        cards.body("fire"),
        cards.chip_recovery(40),
    },
}

return { flameman = card }
