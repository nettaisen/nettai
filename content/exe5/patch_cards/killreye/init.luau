--!strict
-- The patch card KillrEye (20, キラーズアイ in the Japanese games), 21 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 21,
    effects = {
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_paralyzing),
        cards.bug(cards.hp_drain(4)),
    },
}

return { killreye = card }
