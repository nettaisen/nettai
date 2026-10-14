--!strict
-- The patch card BassBX (111, フォルテBX in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")
local rapid_buster = require("../lib/patch_cards/rapid_buster")
local triple_buster = require("../lib/patch_cards/triple_buster")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.super_armor(true),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.b_button(triple_buster),
        cards.charged_shot(rapid_buster),
    },
}

return { bassbx = card }
