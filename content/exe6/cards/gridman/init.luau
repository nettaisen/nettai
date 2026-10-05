--!strict
-- The patch card GridMan (101, フットマン in the Japanese games), 48 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "gridman",
    mb = 48,
    effects = {
        cards.hp_percent(25),
        cards.super_armor(true),
        cards.undershirt(true),
        cards.bug(cards.gauge("slow")),
    },
}
