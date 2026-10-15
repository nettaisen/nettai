--!strict
-- The patch card Bass-Cross MegaMan (111, フォルテクロス ロックマン in the Japanese games), 70 MB. The one card the two teams' card
-- tables differ in: the same number, name and MB, and each team's ROMs their own effects, an effect only one team's
-- card has marked by its version: its two variants, of which a build chooses one for the card (docs/design/versions.md
-- §2.1). Switched on, MegaMan is in Hub Style (0x08137A58), each team's routine writing its own value.

local cards = require("../rules/patch_cards/cards")
local buster = require("../navis/megaman/weapons/buster")
local hells_rolling = require("../navis/megaman/weapons/hells_rolling")
local rapid_buster = require("../navis/megaman/weapons/rapid_buster")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.version("protoman", cards.hp(200)),
        cards.version("colonel", cards.hp_percent(20)),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
        cards.version("protoman", cards.super_armor(true)),
        cards.version("colonel", cards.float_shoes(true)),
        cards.b_button(buster.tri_buster),
        cards.version("protoman", cards.charged_shot(rapid_buster)),
        cards.version("colonel", cards.charged_shot(hells_rolling)),
        cards.version("protoman", cards.hub_style(1)),
        cards.version("colonel", cards.hub_style(2)),
    },
}

return { ["bass-cross-megaman"] = card }
