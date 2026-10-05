--!strict
-- EXE5's panel changer (effect object #0x1F, EXE6's code: @exelib/panel-changer/panel_changer), EXE5's kind; the
-- sounds most changes make EXE6's (a tick every fourth tick, and the change).

local common = require("@exelib/panel-changer")

local panel_changer = {}

-- A change (a "panel-change" record).
panel_changer.change = common.change

panel_changer.TICK_SOUND = asset.sound("panel-change-tick")
panel_changer.CHANGE_SOUND = asset.sound("panel-change")

panel_changer.kind = common.make_kind("panel-changer")

-- A changer of `change` on panel (`x`, `y`), on `spawner`'s side and facing; `holder` goes on once it is done.
function panel_changer.spawn(spawner: Object, x: number, y: number, change: PanelChange, holder: Object?): Object?
    return common.spawn(panel_changer.kind, spawner, x, y, change, holder)
end

return panel_changer
