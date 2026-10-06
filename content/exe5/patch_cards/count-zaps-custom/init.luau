--!strict
-- The patch card Count-Zap's Custom (48, エレキはくしゃく のカスタマイズ in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 35,
    effects = {
        cards.hp(300),
        cards.body("elec"),
    },
}

return { ["count-zaps-custom"] = card }
