--!strict
-- The patch card Soul Battlr's Custom (109, ソウルバトラーの カスタマイズ in the Japanese games), 66 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 66,
    effects = {
        cards.super_armor(true),
        cards.attack(4),
        cards.rapid(4),
        cards.charge(4),
        cards.charged_shot(charge),
    },
}

return { ["soul-battlrs-custom"] = card }
