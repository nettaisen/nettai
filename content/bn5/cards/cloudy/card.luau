--!strict
-- The patch card Cloudy (55, クモンペ in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:cloudy",
    mb = 10,
    effects = {
        cards.body("aqua"),
        cards.custom_damage(0),
        cards.custom_hp_drain(0),
        cards.bug(cards.super_armor(false)),
    },
}
