--!strict
-- The patch card Life-Virus (101, ドリームウイルス in the Japanese games), 55 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers")

return define.patch_card {
    id = "bn5:life-virus",
    mb = 55,
    effects = {
        cards.hp_percent(10),
        cards.first_barrier(barriers.life_aura),
        cards.bug(cards.custom_damage(100)),
    },
}
