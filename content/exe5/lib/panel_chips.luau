--!strict
-- EXE5's panel chips' dimming controller (effect object #0x20, EXE6's code: @exelib/panel_chips/controller), EXE5's
-- kind with EXE5's panel changer (objects/panel_changer).

local common = require("@exelib/panel_chips/controller")
local panel_changer = require("../objects/panel_changer")

local panel_chips = {}

local made = common.make { id = "panel-chips/controller", changer = panel_changer.kind }
panel_chips.kind = made.kind
panel_chips.hook = made.hook

-- The conditions most changes use.
panel_chips.OWN_AREA = common.OWN_AREA
panel_chips.OTHER_AREA = common.OTHER_AREA
panel_chips.ANY = common.ANY

return panel_chips
