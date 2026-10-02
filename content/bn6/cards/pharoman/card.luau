--!strict
-- The patch card PharoMan (87, ファラオマン in the Japanese games), 35 MB.

local cards = require("../../rules/patch-cards/cards")
local chrgs = require("../../lib/patch-cards/chrgs")

return cards.card {
    id = "patch-card/pharoman",
    mb = 35,
    effects = {
        cards.hp_percent(5),
        cards.float_shoes(true),
        cards.bug(cards.charged_shot(chrgs)),
        cards.giga_folder(1),
    },
}
