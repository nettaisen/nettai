--!strict
-- EXE4's AntiRecv counterattack (effect object 0x2C, 0x080E538C: EXE6's code, @exelib/antirecv/controller), EXE4's kind
-- with EXE4's panel changer and bubble, and the changer's row 8 (0x080E4568's: the own panel to poison, 64 ticks,
-- flickering, with EXE4's tick and poison sounds). It takes no mood (0x080E53D0). The rules spawn it by role
-- (`kinds.anti_recovery`, rules/roles.luau: 0x080E5436) in a recovery's place when the healer's other side has AntiRecv
-- armed.

local common = require("@exelib/antirecv/controller")
local panel_changer = require("../../objects/panel_changer")
local rising_bubble = require("../../objects/rising_bubble")

return common.make {
    id = "antirecv/controller",
    changer = panel_changer.kind,
    bubble = rising_bubble.kind,
    tick_sound = panel_changer.TICK_SOUND,
    poison_sound = panel_changer.POISON_SOUND,
}
