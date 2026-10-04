--!strict
-- BN5's AntiRecv's counterattack (effect object #0x2C, BN6's code: @exelib/antirecv/controller), BN5's kind with
-- BN5's panel changer and bubble; the change's sounds BN6's. The ruleset spawns it by role (`kinds.anti_recovery`,
-- rules/roles.luau).

local common = require("@exelib/antirecv/controller")
local panel_changer = require("../../objects/panel-changer/panel_changer")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

return common.make {
    id = "bn5:antirecv/controller",
    changer = panel_changer.kind,
    bubble = rising_bubble.kind,
    tick_sound = asset.sound("bn6:panel-change-tick"),
    poison_sound = asset.sound("bn6:panel-poison"),
}
