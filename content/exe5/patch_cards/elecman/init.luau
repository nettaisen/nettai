--!strict
-- The patch card ElecMan (82, エレキマン in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 35,
    effects = {
        cards.hp(200),
        cards.body("elec"),
        cards.auto_heal(true),
        cards.bug(cards.back_special(nil)),
    },
}

return { elecman = card }
