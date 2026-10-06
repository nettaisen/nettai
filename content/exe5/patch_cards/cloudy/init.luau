--!strict
-- The patch card Cloudy (55, クモンペ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.body("aqua"),
        cards.bug(cards.super_armor(false)),
        cards.custom_damage(0),
        cards.custom_hp_drain(0),
    },
}

return { cloudy = card }
