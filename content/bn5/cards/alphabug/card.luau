--!strict
-- The patch card AlphaBug (71, プロトバグ in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:alphabug",
    mb = 16,
    effects = {
        cards.chip_recovery(60),
        cards.bug(cards.hp_drain(2)),
    },
}
