--!strict
-- The patch card Elebee (70, スパークビー in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_blinding),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.bug(cards.first_barrier(nil)),
    },
}

return { elebee = card }
