--!strict
-- The patch card WoodMan (81, ウッドマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")
local charged_shot = require("../navis/megaman/weapons/charged_shot")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.hp_percent(25),
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.buster_shot(nil)),
        cards.bug(cards.charged_shot(charged_shot.program)),
    },
}

return { woodman = card }
