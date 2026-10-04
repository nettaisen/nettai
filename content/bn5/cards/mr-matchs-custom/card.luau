--!strict
-- The patch card Mr.-Match's Custom (47, ヒノケンの カスタマイズ in the Japanese games), 35 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:mr-matchs-custom",
    mb = 35,
    effects = {
        cards.hp(300),
        cards.body("fire"),
    },
}
