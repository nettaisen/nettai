--!strict
-- The patch card RedUFO (64, UFOサニー in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.charge_shot_kind(variants.chip_destruction),
        cards.bug(cards.chip_recovery(0)),
        cards.bug(cards.step_bug(1)),
    },
}

return { redufo = card }
