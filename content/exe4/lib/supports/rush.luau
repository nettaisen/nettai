--!strict
-- Rush (EXE4's actor object #0x4B, 0x080CA4C4): the shared Rush (@exelib/supports/rush, EXE6's code) in EXE4's look, the
-- NaviCust support that stops a chip Rush cancels (0x0800C8C0: of EXE4's chips, Invis and PopUp). As EXE5's, his bite
-- (0x080CA662) spares no chip and, without the opponent's navi, skips the hand. EXE4's own: he comes up under the
-- opponent's navi on its panel exactly (0x080CA60C adds no pixel to Y and Z), and the panel his navi must stand on for
-- him to appear there (0x080CA6F0) reads its forbidden flags two bytes into the table's row (`ldr` at +2, which the ARM
-- rotates: the row's word turned by 16 bits): side 0's 0x00000001, side 1's 0x00200001, where EXE6's are 0x20 and 0. See
-- docs/engine/chips.md §2.10.

local collision = require("../../rules/collision")
local common = require("@exelib/supports/rush")
local statuses = require("../../rules/status")

return common.make {
    id = "rush",
    sprite = asset.sprite("rush"),
    appear_sound = asset.sound("place"),
    bite_sound = asset.sound("bite"),
    -- Digging in and out (effect row 21, 0x080E1698: the puff's animation 2).
    dust = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    is = collision.thrown_piercing_break,
    hits = collision.hits_navis,
    status = statuses.paralyze_150,
    spared = nil,
    lost_target = "skips",
    -- 0x080CA738's rows (EXE6's words) as 0x080CA70A reads them.
    stand = {
        [0] = { require = 0x0001_0000, forbid = 0x0000_0001 },
        [1] = { require = 0x0001_0020, forbid = 0x0020_0001 },
    },
    pixel_nudge = false,
}
