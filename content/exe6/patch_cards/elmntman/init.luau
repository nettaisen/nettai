--!strict
-- The patch card ElmntMan (106, エレメントマン in the Japanese games), 33 MB.

local cards = require("../rules/patch_cards/cards")
local chrgs = require("../lib/patch_cards/chrgs")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 33,
    effects = {
        cards.hp(150),
        cards.buster_shot(variants.grass),
        cards.charge_shot_kind(variants.charged_ice),
        cards.bug(cards.charged_shot(chrgs)),
        cards.panel_trail("none", 0),
    },
}

return { elmntman = card }
