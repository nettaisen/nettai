--!strict
-- The patch card Soul Battlr's Custom (109, ソウルバトラーの カスタマイズ in the Japanese games), 66 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:soul-battlrs-custom",
    mb = 66,
    effects = {
        cards.super_armor(true),
        cards.attack(4),
        cards.rapid(4),
        cards.charge(4),
        cards.charged_shot(charge),
    },
}
