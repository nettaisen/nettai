--!strict
-- The patch card Count-Zap's Custom (48, エレキはくしゃく のカスタマイズ in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "count-zaps-custom",
    mb = 35,
    effects = {
        cards.hp(300),
        cards.body("elec"),
    },
}
