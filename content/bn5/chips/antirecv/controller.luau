--!strict
-- BN5's AntiRecv's counterattack (effect object #0x2C, BN6's code: @exelib/antirecv/controller), BN5's kind with
-- BN5's panel changer and bubble; the change's sounds BN6's. The ruleset spawns it by role (`kinds.anti_recovery`,
-- rules/roles.luau). BN5's takes the healer's mood too (0x080E39DC: by its damage word's high half, 0x08012820).

local common = require("@exelib/antirecv/controller")
local panel_changer = require("../../objects/panel-changer/panel_changer")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

return common.make {
    id = "antirecv/controller",
    changer = panel_changer.kind,
    bubble = rising_bubble.kind,
    tick_sound = asset.sound("panel-change-tick"),
    poison_sound = asset.sound("panel-poison"),
    takes_mood = true,
}
