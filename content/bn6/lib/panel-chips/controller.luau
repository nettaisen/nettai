--!strict
-- The panel chips' dimming controller (the original's effect object
-- #0x20, `sub_80E2AE8`): the shared controller
-- (@exelib/panel-chips/controller), BN6's kind with BN6's panel changer
-- (objects/panel-changer). A chip's `dimming` hook is
-- `panel_chips.hook(change)`, with its change.

local common = require("@exelib/panel-chips/controller")
local panel_changer = require("../../objects/panel-changer/panel_changer")

local panel_chips = {}

local made = common.make { id = "bn6:panel-chips/controller", changer = panel_changer.kind }
panel_chips.kind = made.kind
panel_chips.hook = made.hook

-- The conditions most changes use (@exelib/panel-chips/controller).
panel_chips.OWN_AREA = common.OWN_AREA
panel_chips.OTHER_AREA = common.OTHER_AREA
panel_chips.ANY = common.ANY

return panel_chips
