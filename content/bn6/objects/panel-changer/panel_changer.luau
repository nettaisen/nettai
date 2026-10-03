--!strict
-- The panel changer (the original's effect object #0x1F, `sub_80E28A8`;
-- docs/engine/dimming-chips.md §4.2): it lists the panels its change picks
-- (all, its row, the panel in front, or its own; those meeting its side's
-- condition), shows them as the new type while its timer runs, then
-- changes them, and lets its holder go on. What changes is a record
-- (`panel_changer.change { ... }`, a row of the original's `byte_80E272C`),
-- defined by its users: the panel chips (lib/panel-chips: PnlRetrn,
-- HolyPanl, Snctuary, ComingRd, GoingRd) and AntiRecv's counterattack
-- (chips/antirecv/controller).

local panels = require("@common/panels")

local panel_changer = {}

export type State = {
    change: PanelChange,
    -- The panels listed (the object's +0x68, packed y << 4 | x), and how
    -- many (ExtraVars+4).
    list: { number },
    count: number,
    -- The object whose `busy` it clears when done (the original's pointer
    -- to its spawner's Param2, whose word store clears its parameters).
    holder: Object?,
}

-- A change: which panels (`off_80E291C`), the type they become, how they
-- show meanwhile, how long, and the conditions by side. (`byte_80E272C`'s
-- rows by kind: its sounds were the kind's, now the row's.)
function panel_changer.change(c: PanelChange): PanelChange
    return define.record("panel-change", c)
end

-- The sounds most changes make: a tick every fourth tick, and the change.
panel_changer.TICK_SOUND = asset.sound("bn6:panel-change-tick")
panel_changer.CHANGE_SOUND = asset.sound("bn6:panel-change")
panel_changer.POISON_SOUND = asset.sound("bn6:panel-poison")

-- `sub_80E2ACA`: a changer of `change` on panel (`x`, `y`), on `spawner`'s
-- side and facing; `holder` (an object with a boolean `busy` its spawner
-- set) goes on once it is done. (It is spawned at the spawner's registers:
-- X the panel's Y, Y and Z garbage, which nothing reads.)
function panel_changer.spawn(spawner: Object, x: number, y: number, change: PanelChange, holder: Object?): Object?
    local o = battle.spawn(panel_changer.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.change, s.holder = change, holder
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = spawner.alliance, spawner.flip
    end
    return o
end

-- The type the panels become on the changer's side.
local function new_type(me: Object, c: PanelChange): PanelType
    if me.alliance ~= 0 and c.side1_panel then
        return c.side1_panel
    end
    return c.panel
end

-- `off_80E291C`: the panels the change picks, meeting the side's
-- condition. (`sub_80E29F8`: rows 3 to 1, columns 6 to 1; `sub_80E2A30`:
-- its row, columns 6 to 1; `sub_80E2A64`: the panel in front;
-- `sub_80E2A9A`: its own.)
local function collect(me: Object, c: PanelChange): { number }
    local candidates: { number } = {}
    if c.panels == "all" then
        for y = 3, 1, -1 do
            for x = 6, 1, -1 do
                candidates[#candidates + 1] = panels.pack(x, y)
            end
        end
    elseif c.panels == "row" then
        for x = 6, 1, -1 do
            candidates[#candidates + 1] = panels.pack(x, me.panel_y)
        end
    elseif c.panels == "front" then
        candidates[#candidates + 1] = panels.pack(me.panel_x + me.facing, me.panel_y)
    else
        candidates[#candidates + 1] = panels.pack(me.panel_x, me.panel_y)
    end
    local cond = c.sides[me.alliance]
    local list: { number } = {}
    for _, p in candidates do
        if field.check(panels.x(p), panels.y(p), cond.require, cond.forbid) then
            table.insert(list, p)
        end
    end
    return list
end

-- Each listed panel, last to first.
local function each_panel(me: Object, f: (x: number, y: number) -> ())
    local s = me.state :: State
    for i = s.count, 1, -1 do
        local p = s.list[i]
        f(panels.x(p), panels.y(p))
    end
end

local function side_of(x: number, y: number): number
    local p = field.panel(x, y)
    return if p then p.alliance else 0
end

-- `sub_80E292C`: count down, showing the panels as their new type
-- meanwhile (`object_setPanelTypeBlink`: every tick, or when the change
-- flickers only on ticks whose timer has bit 2); at 0 change them, and go.
local function tick(me: Object)
    local c = (me.state :: State).change
    local t = me.timer - 1
    me.timer = t
    local new = new_type(me, c)
    if t ~= 0 then
        if bit32.band(t, 3) == 0 and c.tick_sound then
            battle.play_sound(c.tick_sound)
        end
        if c.flicker and not bit32.btest(t, 4) then
            return
        end
        each_panel(me, function(x, y)
            field.blink(x, y, new, side_of(x, y))
        end)
        return
    end
    each_panel(me, function(x, y)
        local side = side_of(x, y)
        field.set_type(x, y, new)
        field.blink(x, y, new, side)
    end)
    if c.change_sound then
        battle.play_sound(c.change_sound)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80E28C8`: list the panels; the timer; and the first update at once.
local function init(me: Object)
    local s = me.state :: State
    local list = collect(me, s.change)
    for i, p in list do
        s.list[i] = p
    end
    s.count = #list
    me.timer = s.change.ticks
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80E29E6`: the holder goes on, and so does the changer.
local function finish(me: Object)
    local holder = (me.state :: State).holder
    if holder then
        (holder.state :: { busy: boolean }).busy = false
    end
    me:free()
end

panel_changer.kind = define.kind {
    id = "bn6:panel-changer",
    pool = "effect",
    state = { change = "record:panel-change", list = "u8[18]", count = "u8", holder = "object" },
    -- `sub_80E28A8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            finish(me)
        end
    end,
}

return panel_changer
