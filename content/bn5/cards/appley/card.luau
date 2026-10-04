--!strict
-- The patch card Appley (80, アップルサム in the Japanese games), 24 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "appley",
    mb = 24,
    effects = {
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.buster_blanks(1, 2)),
        cards.bug(cards.emotion_bug(1)),
    },
}
