--!strict
-- The patch card FlameMan (38, フレイムマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "flameman",
    mb = 45,
    effects = {
        cards.hp_percent(10),
        cards.body("fire"),
        cards.chip_recovery(40),
    },
}
