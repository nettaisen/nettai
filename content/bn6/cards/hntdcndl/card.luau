--!strict
-- The patch card HntdCndl (75, ドルダーラ in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards/cards")
local chrgs = require("../../lib/patch-cards/chrgs")

return define.patch_card {
    id = "bn6:hntdcndl",
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("fire"),
        cards.bug(cards.charged_shot(chrgs)),
        cards.hp_drain(0),
    },
}
