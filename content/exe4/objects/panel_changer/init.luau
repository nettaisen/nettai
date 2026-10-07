--!strict
-- EXE4's panel changer (effect object #0x1F, 0x080E4354; EXE6's code, @exelib/panel_changer, but for its sounds): a
-- change is a row of 0x080E423C (20 bytes: the new type, which panels, whether they flicker, the ticks, and each side's
-- condition), which its users define. Its sounds go by the row's number rather than the row: a change to holes (rows
-- 9, 10 and 12) starts with `START_SOUND` (0x105, 0x080E4390); every fourth tick sounds `TICK_SOUND` (0x106) but in
-- the Hole chip's rows 9 and 10; the change sounds `POISON_SOUND` (0x124) in the poison rows 8 and 13, none in rows 9
-- and 10, else `CHANGE_SOUND` (0x107, 0x080E4460). EXE4 has no second panel type for side 1.

local common = require("@exelib/panel_changer")

local panel_changer = {}

-- A change (a "panel-change" record).
panel_changer.change = common.change

panel_changer.START_SOUND = asset.sound("panel-hole")
panel_changer.TICK_SOUND = asset.sound("panel-change-tick")
panel_changer.CHANGE_SOUND = asset.sound("panel-change")
panel_changer.POISON_SOUND = asset.sound("panel-poison")

-- The conditions EXE4's rows use: a side's own area (side 0's: not missing, not side 1's; side 1's: side 1's, not
-- missing), the other side's, and any panel that isn't missing.
panel_changer.OWN_AREA = { [0] = { require = 0x0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } }
panel_changer.OTHER_AREA = { [0] = { require = 0x20, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8020 } }
panel_changer.ANY = { [0] = { require = 0x0, forbid = 0x8000 }, [1] = { require = 0x0, forbid = 0x8000 } }

panel_changer.kind = common.make_kind("panel-changer")

-- 0x080E4568: a changer of `change` on panel (`x`, `y`), on `spawner`'s side (EXE4's takes no facing of it);
-- `holder` goes on once it is done. (It is spawned at the spawner's registers, which nothing reads.)
function panel_changer.spawn(spawner: Object, x: number, y: number, change: PanelChange, holder: Object?): Object?
    local o = battle.spawn(panel_changer.kind, Vec3.zero)
    if o then
        local s = o.state :: { change: PanelChange, holder: Object? }
        s.change, s.holder = change, holder
        o.panel_x, o.panel_y = x, y
        o.alliance = spawner.alliance
    end
    return o
end

return panel_changer
