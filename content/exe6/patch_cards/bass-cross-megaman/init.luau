--!strict
-- The patch card Bass-Cross MegaMan (117, フォルテクロス ロックマン in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")
local triple_buster = require("../lib/patch_cards/triple_buster")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.hp_percent(20),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
        cards.float_shoes(true),
        cards.b_button(triple_buster),
        cards.charged_shot(charge),
    },
}

return { ["bass-cross-megaman"] = card }
