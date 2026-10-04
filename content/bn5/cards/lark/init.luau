--!strict
-- The patch card Lark (23, ゲイラーク in the Japanese games), 17 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "lark",
    mb = 17,
    effects = {
        cards.hp(100),
        cards.body("aqua"),
        cards.charged_shot(chips.widesht),
        cards.bug(cards.hit_status("blind")),
    },
}
