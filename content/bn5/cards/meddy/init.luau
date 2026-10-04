--!strict
-- The patch card Meddy (44, メディ in the Japanese games), 45 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "meddy",
    mb = 45,
    effects = {
        cards.chip_recovery(50),
        cards.tango(true),
        cards.hit_status("none"),
        cards.bug(cards.custom_hp_drain(1)),
    },
}
