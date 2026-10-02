--!strict
-- The patch card Dark MegaMan (108, ダークロックマン in the Japanese games), 80 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/dark-megaman",
    name = "Dark MegaMan",
    mb = 80,
    effects = {
        cards.attack(3),
        cards.buster_shot(variants.poisoning),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
        cards.bug(cards.hp_drain(4)),
    },
}
