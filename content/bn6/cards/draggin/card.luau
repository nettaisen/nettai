--!strict
-- The patch card Draggin (73, ドラグリン in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "draggin",
    mb = 11,
    effects = {
        cards.hp_percent(5),
        cards.body("fire"),
        cards.bug(cards.status_bug(1)),
    },
}
