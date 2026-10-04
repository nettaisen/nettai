--!strict
-- The patch card Gospel (102, ゴスペル in the Japanese games), 60 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "gospel",
    mb = 60,
    effects = {
        cards.hp(600),
        cards.bug(cards.emotion_bug(1)),
        cards.bug(cards.hp_drain(2)),
        cards.bug(cards.custom_hp_drain(2)),
        cards.bug(cards.status_bug(2)),
    },
}
