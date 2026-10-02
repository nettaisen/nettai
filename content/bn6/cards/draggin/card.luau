--!strict
-- The patch card Draggin (73, ドラグリン in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/draggin",
    name = "Draggin",
    mb = 11,
    effects = {
        cards.hp_percent(5),
        cards.body("fire"),
        cards.bug(cards.status_bug(1)),
    },
}
