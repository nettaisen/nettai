--!strict
-- The patch card HntdCndl (75, ドルダーラ in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards/cards")
local chrgs = require("../../lib/patch-cards/chrgs")

return cards.card {
    id = "patch-card/hntdcndl",
    name = "HntdCndl",
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("fire"),
        cards.bug(cards.charged_shot(chrgs)),
        cards.hp_drain(0),
    },
}
