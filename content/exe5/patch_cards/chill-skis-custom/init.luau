--!strict
-- The patch card Chill-ski's Custom (50, コオリスキーの カスタマイズ in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 47,
    effects = {
        cards.hp_percent(30),
        cards.body("aqua"),
    },
}

return { ["chill-skis-custom"] = card }
