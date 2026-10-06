--!strict
-- The patch card Chill-ski's Custom (50, コオリスキーの カスタマイズ in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "chill-skis-custom",
    mb = 47,
    effects = {
        cards.hp_percent(30),
        cards.body("aqua"),
    },
}
