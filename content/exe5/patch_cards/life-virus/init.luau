--!strict
-- The patch card Life-Virus (101, ドリームウイルス in the Japanese games), 55 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    mb = 55,
    effects = {
        cards.hp_percent(10),
        cards.first_barrier(barriers.life_aura),
        cards.bug(cards.custom_damage(100)),
    },
}

return { ["life-virus"] = card }
