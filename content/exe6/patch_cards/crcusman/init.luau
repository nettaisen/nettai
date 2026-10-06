--!strict
-- The patch card CrcusMan (53, サーカスマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 43,
    effects = {
        cards.bug(cards.hp(-200)),
        cards.giga_folder(1),
        cards.bug(cards.first_barrier(nil)),
        cards.tango(true),
    },
}

return { crcusman = card }
