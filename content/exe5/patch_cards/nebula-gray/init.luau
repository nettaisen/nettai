--!strict
-- The patch card Nebula-Gray (108, ネビュラグレイ in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.charged_shot(chips.drksword),
        cards.bug(cards.hp_percent(-30)),
        cards.bug(cards.hp_drain(2)),
        cards.bug(cards.custom_hp_drain(2)),
    },
}

return { ["nebula-gray"] = card }
