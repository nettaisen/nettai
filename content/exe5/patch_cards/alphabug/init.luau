--!strict
-- The patch card AlphaBug (71, プロトバグ in the Japanese games), 16 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 16,
    effects = {
        cards.chip_recovery(60),
        cards.bug(cards.hp_drain(2)),
    },
}

return { alphabug = card }
