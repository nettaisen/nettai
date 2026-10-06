--!strict
-- The patch card Heavy (10, ヘビーアレイ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.attack(1),
        cards.super_armor(true),
        cards.bug(cards.buster_shot(nil)),
    },
}

return { heavy = card }
