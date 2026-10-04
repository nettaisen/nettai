--!strict
-- The patch card GateMan (87, ゲートマン in the Japanese games), 46 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "gateman",
    mb = 46,
    effects = {
        cards.hp(150),
        cards.custom(2),
        cards.bug(cards.gauge("normal")),
    },
}
