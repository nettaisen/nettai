--!strict
-- AntiRecv's counterattack (the original's effect object #0x2C,
-- `sub_80E3728`): the shared counterattack (@exelib/antirecv/controller),
-- BN6's kind with BN6's panel changer and bubble. The ruleset spawns it by
-- role (`kinds.anti_recovery`, rules/roles.luau).

local common = require("@exelib/antirecv/controller")
local panel_changer = require("../../objects/panel-changer/panel_changer")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

return common.make {
    id = "antirecv/controller",
    changer = panel_changer.kind,
    bubble = rising_bubble.kind,
    tick_sound = panel_changer.TICK_SOUND,
    poison_sound = panel_changer.POISON_SOUND,
}
