--!strict
-- The patch card Heavy (10, ヘビーアレイ in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/heavy",
    mb = 15,
    effects = {
        cards.super_armor(true),
        cards.attack(1),
        cards.bug(cards.buster_shot(nil)),
    },
}
