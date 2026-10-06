--!strict
-- The patch card ProtoMan (83, ブルース in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")
local guards = require("../navis/megaman/weapons/guards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.hp(100),
        cards.charged_shot(chips.wideblde),
        cards.back_special(guards.shield),
    },
}

return { protoman = card }
