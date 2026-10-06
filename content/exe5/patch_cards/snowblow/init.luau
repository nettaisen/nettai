--!strict
-- The patch card SnowBlow (19, スーンハーク in the Japanese games), 17 MB.

local cards = require("../rules/patch_cards/cards")
local instants = require("../navis/megaman/weapons/instants")

local card: PatchCard = {
    mb = 17,
    effects = {
        cards.hp(120),
        cards.charged_shot(instants.vacuum),
        cards.bug(cards.gauge("slow")),
    },
}

return { snowblow = card }
