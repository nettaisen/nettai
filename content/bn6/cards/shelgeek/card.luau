--!strict
-- The patch card ShelGeek (62, シェルキー in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards/cards")
local shield = require("../../navis/megaman/weapons/shield/weapon")

return cards.card {
    id = "patch-card/shelgeek",
    name = "ShelGeek",
    mb = 8,
    effects = {
        cards.body("aqua"),
        cards.rapid(4),
        cards.back_special(shield.shield),
    },
}
