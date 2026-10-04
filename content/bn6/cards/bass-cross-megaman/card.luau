--!strict
-- The patch card Bass-Cross MegaMan (117, フォルテクロス ロックマン in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")
local triple_buster = require("../../lib/patch-cards/triple-buster")

return define.patch_card {
    id = "bass-cross-megaman",
    mb = 70,
    effects = {
        cards.hp_percent(20),
        cards.float_shoes(true),
        cards.b_button(triple_buster),
        cards.charged_shot(charge),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
    },
}
