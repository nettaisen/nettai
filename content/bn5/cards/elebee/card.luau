--!strict
-- The patch card Elebee (70, スパークビー in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local charged_shot = require("../../navis/megaman/weapons/charged-shot")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "elebee",
    mb = 20,
    effects = {
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_blinding),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.bug(cards.first_barrier(nil)),
    },
}
