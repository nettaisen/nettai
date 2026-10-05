--!strict
-- The patch card Chaud's Custom (46, 炎山の カスタマイズ in the Japanese games), 35 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "chauds-custom",
    mb = 35,
    effects = {
        cards.b_button(chips.sword),
        cards.charged_shot(chips.wideswrd),
        cards.custom(1),
    },
}
