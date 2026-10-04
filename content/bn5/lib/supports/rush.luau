--!strict
-- Rush (BN5's actor object #0x4B, 0x080C2214): the shared Rush (@common/supports/rush, BN6's code) in BN5's look, the
-- NaviCust support that stops a chip Rush cancels (0x0800E498: of BN5's chips, Invisibl). BN5's bite (0x080C23BE)
-- spares no chip: BN6's check for the second WhiCapsl isn't there, and BN5's ruleset hands Rush no chip (0x0800E498
-- passes 0). Without the opponent's navi at his bite, BN5's branch skips the hand where BN6's unbalances the stack. See
-- docs/engine/chips.md §2.10.

local collision = require("../../rules/collision")
local common = require("@common/supports/rush")
local statuses = require("../../rules/status")

return common.make {
    id = "bn5:rush",
    sprite = asset.sprite("bn5:rush"),
    appear_sound = asset.sound("bn5:place"),
    bite_sound = asset.sound("bn5:bite"),
    -- Digging in and out (BN5's effect 0x14).
    dust = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 },
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    status = statuses.paralyze_150,
    spared = nil,
    lost_target = "skips",
}
