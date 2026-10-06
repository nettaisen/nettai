--!strict
-- The patch card Dex's Custom (97, デカオの カスタマイズ in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "dexs-custom",
    mb = 40,
    effects = {
        cards.hp(150),
        cards.attack(5),
    },
}
