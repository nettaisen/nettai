--!strict
-- The patch card Anetta's Custom (49, アネッタの カスタマイズ in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 47,
    effects = {
        cards.hp_percent(30),
        cards.body("wood"),
    },
}

return { ["anettas-custom"] = card }
