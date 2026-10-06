--!strict
-- Rush (EXE5's actor object #0x4B, 0x080C2214): the shared Rush (@exelib/supports/rush, EXE6's code) in EXE5's look, the
-- NaviCust support that stops a chip Rush cancels (0x0800E498: of EXE5's chips, Invisibl). EXE5's bite (0x080C23BE)
-- spares no chip: EXE6's check for the second WhiCapsl isn't there, and EXE5's rules hand Rush no chip (0x0800E498
-- passes 0). Without the opponent's navi at his bite, EXE5's branch skips the hand where EXE6's unbalances the stack. See
-- docs/engine/chips.md §2.10.

local collision = require("../../rules/collision")
local common = require("@exelib/supports/rush")
local statuses = require("../../rules/status")

return common.make {
    id = "rush",
    sprite = asset.sprite("rush"),
    appear_sound = asset.sound("place"),
    bite_sound = asset.sound("bite"),
    -- Digging in and out (EXE5's effect 0x14).
    dust = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    status = statuses.paralyze_150,
    spared = nil,
    lost_target = "skips",
}
