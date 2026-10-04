--!strict
-- The TankCans' action (the original's action 0x24, `sub_80ECACA`;
-- docs/engine/shot-chips.md §7): the shared action
-- (@exelib/tankcan/action) with BN6's tank cannon and shell (shell.luau).
-- The navi aims its tank cannon for 16 ticks, then fires a shell from the
-- panel ahead on the 23rd, the screen shaking; it lowers the cannon on the
-- 36th and is back to idle 4 ticks later.

local attachment = require("../../objects/attachment/attachment")
local common = require("@exelib/tankcan/action")
local shell = require("./shell")

local tankcan = {}

-- One state for the three.
tankcan.STATE = common.STATE

-- The tank cannon (the original's attachment row 0x14).
local TANK = attachment.look { sprite = asset.sprite("tank-cannon"), palette = 0, attach_point = 8 }

-- A chip's tank cannon: the shell it fires.
function tankcan.action(spec: { shell: ShellSpec }): Action
    return common.action { cannon = TANK, attachment = attachment.kind, kind = shell.kind, shell = spec.shell }
end

return tankcan
