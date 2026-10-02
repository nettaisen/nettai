--!strict
-- The patch card Metrid (64, メテマージ in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local back_special = require("./back-special")

return cards.card {
    id = "patch-card/metrid",
    name = "Metrid",
    mb = 15,
    effects = {
        cards.hp_percent(5),
        cards.body("fire"),
        cards.back_special(back_special),
        cards.bug(cards.step_bug(1)),
    },
}
