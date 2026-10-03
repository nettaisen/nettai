--!strict
-- GrabBnsh's and GrabRvng's dimming controller (the original's effect
-- object #0x22, `sub_80E2CA4`): the shared controller
-- (@common/grabbnsh/controller), BN6's kind with BN6's hand (./hand), the
-- flash on each panel taken back (effect 0x3B) and its sound.

local common = require("@common/grabbnsh/controller")
local hand = require("./hand")

return common.make {
    id = "bn6:grabbnsh/controller",
    hand = hand.kind,
    returned = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 0 },
    return_sound = asset.sound("bn6:err-select-91"),
}
