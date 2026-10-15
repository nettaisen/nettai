--!strict
-- The patch card Bass-Cross MegaMan (117, フォルテクロス ロックマン in the Japanese games), 70 MB. The one card the two
-- versions' card tables differ in: the same number, name and MB, and each version's ROM its own effects (JP Gregar's
-- card table, 0x08144778, and JP Falzar's, 0x081429B0). Both have NormBody, TriBustr and a MegFldr-2 bug; Gregar's
-- has HP+200, SprArmr and the rapid buster (routine 0x92, BassBX's), Falzar's HP+20%, FlotShoe and the seeking
-- whirlwind (routine 0x93). In the card screen's order, each version's. So it has two variants, Gregar's and
-- Falzar's (the versions its effects name), of which a build chooses one for the card, as a NaviCust program's color is
-- chosen (docs/design/versions.md §2.1, §3.1): each the group's of its name (../../rules/builds).

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")
local rapid_buster = require("../lib/patch_cards/rapid_buster")
local triple_buster = require("../lib/patch_cards/triple_buster")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.version("gregar", cards.hp(200)),
        cards.version("falzar", cards.hp_percent(20)),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
        cards.version("gregar", cards.super_armor(true)),
        cards.version("falzar", cards.float_shoes(true)),
        cards.b_button(triple_buster),
        cards.version("gregar", cards.charged_shot(rapid_buster)),
        cards.version("falzar", cards.charged_shot(charge)),
    },
}

return { ["bass-cross-megaman"] = card }
