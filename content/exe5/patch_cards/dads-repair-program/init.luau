--!strict
-- The patch card Dad's Repair Program (109, パパのしゅうせい プログラム in the Japanese games), 50 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 50,
    effects = {
        cards.bug_stopper(1),
    },
}

return { ["dads-repair-program"] = card }
