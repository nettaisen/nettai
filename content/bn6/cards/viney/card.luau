--!strict
-- The patch card Viney (11, アゾマータ in the Japanese games), 24 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/viney",
    mb = 24,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.back_special(nil)),
    },
}
