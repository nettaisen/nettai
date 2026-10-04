--!strict
-- A die's marker over its panel (BN5's effect object #0x83, 0x080E9F38; BN5's own), in the operation battle
-- only: an entry of the 32 the battle's panel numbers draw from (0x0203C51C's bits, their records at 0x02035760),
-- showing its style (1: where the die lands, 15: its countdown) for `ticks` ticks and one more; it goes at once when
-- all 32 are taken. The engine draws none of them: the marker only holds its place in the effects' list.

local operation_battle = require("../operation_battle")

local marker = {}

export type State = {
    -- Its style (Param0), which the drawing reads.
    style: number,
}

-- 0x080E9F58: its entry (always one: nothing else in a battle takes them).
local function init(me: Object)
    me:set_lifecycle("update")
end

-- 0x080E9F88: its ticks run out; its entry is let go (0x0802E950, 0x0802E944), and it goes.
local function tick(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

marker.kind = define.kind {
    id = "dice-marker",
    pool = "effect",
    state = { style = "u8" },
    -- 0x080E9F38.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E9FA4 (`flip` false: of `owner`'s side alone) and 0x080E9FD2 (its facing too): a marker on (x, y) of
-- `style` for `ticks`, in the operation battle. (Its position is the spawner's registers: the row, the ticks
-- and r3, which nothing sets.)
function marker.spawn(owner: Object, x: number, y: number, ticks: number, style: number, flip: boolean): Object?
    if not operation_battle() then
        return nil
    end
    local o = battle.spawn(marker.kind, Vec3.new(y, ticks, 0))
    if o then
        (o.state :: State).style = style
        o.panel_x, o.panel_y = x, y
        o.timer = int.u16(ticks)
        o.alliance = owner.alliance
        if flip then
            o.flip = owner.flip
        end
    end
    return o
end

return marker
