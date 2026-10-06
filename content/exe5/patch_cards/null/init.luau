--!strict
-- The patch card Null (16, ヌール in the Japanese games), 13 MB.

local cards = require("../rules/patch_cards/cards")
local charged_shot = require("../navis/megaman/weapons/charged_shot")

local card: PatchCard = {
    mb = 13,
    effects = {
        cards.hp(150),
        cards.body("aqua"),
        cards.auto_heal(true),
        cards.bug(cards.super_armor(false)),
        cards.bug(cards.charged_shot(charged_shot.program)),
    },
}

return { null = card }
