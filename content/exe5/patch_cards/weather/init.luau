--!strict
-- The patch card Weather (75, ウェザース in the Japanese games), 19 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 19,
    effects = {
        cards.buster_shot(variants.grass),
        cards.charge_shot_kind(variants.charged_lava),
        cards.bug(cards.panel_trail("ice", 4)),
    },
}

return { weather = card }
