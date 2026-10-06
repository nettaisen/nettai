--!strict
-- The patch card Meddy (44, メディ in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.chip_recovery(50),
        cards.tango(true),
        cards.hit_status("none"),
        cards.bug(cards.custom_hp_drain(1)),
    },
}

return { meddy = card }
