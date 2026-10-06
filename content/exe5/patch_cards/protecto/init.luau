--!strict
-- The patch card Protecto (67, プロテクト in the Japanese games), 23 MB.

local cards = require("../rules/patch_cards/cards")
local charged_shot = require("../navis/megaman/weapons/charged_shot")

local card: PatchCard = {
    mb = 23,
    effects = {
        cards.giga_folder(1),
        cards.bug(cards.undershirt(false)),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.bug(cards.back_special(nil)),
    },
}

return { protecto = card }
