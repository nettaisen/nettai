--!strict
-- The patch card GateMan (87, ゲートマン in the Japanese games), 46 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 46,
    effects = {
        cards.hp(150),
        cards.custom(2),
        cards.bug(cards.gauge("normal")),
    },
}

return { gateman = card }
