--!strict
-- The patch card BassBX (111, フォルテBX in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")
local triple_buster = require("../../lib/patch-cards/triple-buster")

return cards.card {
    id = "patch-card/bassbx",
    mb = 70,
    effects = {
        cards.super_armor(true),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.b_button(triple_buster),
        cards.charged_shot(charge),
    },
}
