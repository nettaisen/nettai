--!strict
-- The patch card Yahoot's Bug Repair (99, マハ・ジャラマの バグしゅうせい in the Japanese games), 17 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 17,
    effects = {
        cards.step_bug(0),
        cards.panel_trail("none", 0),
    },
}

return { ["yahoots-bug-repair"] = card }
