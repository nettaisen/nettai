--!strict
-- The patch card Famous' Custom (110, 名人のちょうぜつ カスタマイズ in the Japanese games), 69 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    mb = 69,
    effects = {
        cards.custom(3),
        cards.mega_folder(2),
        cards.giga_folder(2),
        cards.first_barrier(barriers.life_aura),
    },
}

return { ["famous-custom"] = card }
