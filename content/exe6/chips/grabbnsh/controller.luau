--!strict
-- GrabBnsh's and GrabRvng's dimming controller (the original's effect
-- object #0x22, `sub_80E2CA4`): the shared controller
-- (@exelib/grabbnsh/controller), EXE6's kind with EXE6's hand (./hand), the
-- flash on each panel taken back (effect 0x3B) and its sound.

local common = require("@exelib/grabbnsh/controller")
local hand = require("./hand")

return common.make {
    id = "grabbnsh/controller",
    hand = hand.kind,
    returned = define.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 0 },
    return_sound = asset.sound("err-select-91"),
}
