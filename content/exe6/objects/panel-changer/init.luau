--!strict
-- The panel changer (the original's effect object #0x1F, `sub_80E28A8`):
-- the shared changer (@exelib/panel-changer/panel_changer), EXE6's kind and
-- sounds. What changes is a record (`panel_changer.change { ... }`, a row
-- of the original's `byte_80E272C`), defined by its users: the panel chips
-- (lib/panel-chips) and AntiRecv's counterattack (chips/antirecv).

local common = require("@exelib/panel-changer")

local panel_changer = {}

-- A change (a "panel-change" record).
panel_changer.change = common.change

-- The sounds most changes make: a tick every fourth tick, and the change.
panel_changer.TICK_SOUND = asset.sound("panel-change-tick")
panel_changer.CHANGE_SOUND = asset.sound("panel-change")
panel_changer.POISON_SOUND = asset.sound("panel-poison")

panel_changer.kind = common.make_kind("panel-changer")

-- `sub_80E2ACA`: a changer of `change` on panel (`x`, `y`), on `spawner`'s
-- side and facing; `holder` goes on once it is done.
function panel_changer.spawn(spawner: Object, x: number, y: number, change: PanelChange, holder: Object?): Object?
    return common.spawn(panel_changer.kind, spawner, x, y, change, holder)
end

return panel_changer
