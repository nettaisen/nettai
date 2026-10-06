--!strict
-- The patch card Boomer (22, ラウンダ in the Japanese games), 18 MB.

local cards = require("../rules/patch_cards/cards")
local buster = require("../navis/megaman/weapons/buster")
local chips = require("../navis/megaman/weapons/chips")
local guards = require("../navis/megaman/weapons/guards")

local card: PatchCard = {
    mb = 18,
    effects = {
        cards.bug(cards.b_button(buster.weapon)),
        cards.charged_shot(chips.boomer),
        cards.back_special(guards.shield),
        cards.bug(cards.custom_hp_drain(1)),
    },
}

return { boomer = card }
