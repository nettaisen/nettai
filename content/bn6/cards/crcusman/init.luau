--!strict
-- The patch card CrcusMan (53, サーカスマン in the Japanese games), 43 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "crcusman",
    mb = 43,
    effects = {
        cards.bug(cards.hp(-200)),
        cards.bug(cards.first_barrier(nil)),
        cards.giga_folder(1),
        cards.tango(true),
    },
}
