--!strict
-- The patch card Famous' Custom (110, 名人のちょうぜつ カスタマイズ in the Japanese games), 69 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers/barriers")

return cards.card {
    id = "patch-card/famous-custom",
    name = "Famous' Custom",
    mb = 69,
    effects = {
        cards.first_barrier(barriers.life_aura),
        cards.custom(3),
        cards.mega_folder(2),
        cards.giga_folder(2),
    },
}
