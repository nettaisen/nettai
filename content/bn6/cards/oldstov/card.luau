--!strict
-- The patch card OldStov (21, ダルスト in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "oldstov",
    name = "OldStov",
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("fire"),
        cards.charged_shot(charge),
        cards.bug(cards.hp_drain(1)),
    },
}
