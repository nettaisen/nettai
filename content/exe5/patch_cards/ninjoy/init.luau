--!strict
-- The patch card Ninjoy (65, ゲニン in the Japanese games), 18 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 18,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charged_shot(chips.moonbld),
        cards.sneak_run(true),
    },
}

return { ninjoy = card }
