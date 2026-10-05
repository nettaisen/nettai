--!strict
-- Rush (the original's actor object #0x4B, `sub_80C3218`): the shared Rush (@exelib/supports/rush, EXE5's too) in
-- EXE6's look, the NaviCust support that stops a chip Rush cancels (`sub_8010740`). He spares the second WhiCapsl,
-- which doesn't come from the hand; without the opponent's navi at his bite, the game's stack goes wrong. See
-- docs/engine/chips.md §2.10.

local collision = require("../../rules/collision")
local common = require("@exelib/supports/rush")
local effects = require("../effects")
local statuses = require("../../rules/status")
local whicapsl_invisible = require("../../chips/whicapsl-invisible")

return common.make {
    id = "rush",
    sprite = asset.sprite("rush"),
    appear_sound = asset.sound("place"),
    bite_sound = asset.sound("bite"),
    -- Digging in and out.
    dust = effects.puff,
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    status = statuses.paralyze_150,
    -- The chip he leaves in the hand: the second WhiCapsl.
    spared = whicapsl_invisible,
    lost_target = "unbalances",
}
