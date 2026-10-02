--!strict
-- The patch card IceMan (32, アイスマン in the Japanese games), 25 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/iceman",
    name = "IceMan",
    mb = 25,
    effects = {
        cards.hp(80),
        cards.body("aqua"),
        cards.bug(cards.super_armor(false)),
        cards.charged_shot(charge),
        cards.custom_hp_drain(0),
    },
}
