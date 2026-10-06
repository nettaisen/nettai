--!strict
-- The patch card Metrid (64, メテマージ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local back_special = require("@self/back_special")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.hp_percent(5),
        cards.body("fire"),
        cards.back_special(back_special),
        cards.bug(cards.step_bug(1)),
    },
}

return { metrid = card }
