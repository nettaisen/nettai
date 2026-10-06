--!strict
-- The patch card Dex's Custom (97, デカオの カスタマイズ in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(150),
        cards.attack(5),
    },
}

return { ["dexs-custom"] = card }
