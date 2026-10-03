--!strict
-- BN5's panel changer (effect object #0x1F, BN6's code: @common/panel-changer/panel_changer), BN5's kind; the
-- sounds most changes make BN6's (a tick every fourth tick, and the change).

local common = require("@common/panel-changer/panel_changer")

local panel_changer = {}

-- A change (a "panel-change" record).
panel_changer.change = common.change

panel_changer.TICK_SOUND = asset.sound("bn6:panel-change-tick")
panel_changer.CHANGE_SOUND = asset.sound("bn6:panel-change")

panel_changer.kind = common.make_kind("bn5:panel-changer")

-- A changer of `change` on panel (`x`, `y`), on `spawner`'s side and facing; `holder` goes on once it is done.
function panel_changer.spawn(spawner: Object, x: number, y: number, change: PanelChange, holder: Object?): Object?
    return common.spawn(panel_changer.kind, spawner, x, y, change, holder)
end

return panel_changer
