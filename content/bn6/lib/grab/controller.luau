--!strict
-- AreaGrab's and PanelGrab's dimming controller (the original's effect
-- object #3, `sub_80E0710`): the shared controller
-- (@exelib/grab/controller), BN6's kind with BN6's grab shot
-- (lib/grab/shot). Each chip's `dimming` hook is `grab.area` or
-- `grab.panel`.

local common = require("@exelib/grab/controller")
local shot = require("./shot")

return common.make { id = "grab/controller", shot = shot.kind }
