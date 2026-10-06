--!strict
-- The patch card Number (14, ナンバーズ in the Japanese games), 17 MB.

local cards = require("../rules/patch_cards/cards")
local chrgs = require("../lib/patch_cards/chrgs")

local card: PatchCard = {
    mb = 17,
    effects = {
        cards.bug(cards.charged_shot(chrgs)),
        cards.custom(1),
        cards.mega_folder(1),
        cards.giga_folder(1),
        cards.bug(cards.custom_hp_drain(2)),
    },
}

return { number = card }
