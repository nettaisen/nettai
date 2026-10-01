--!strict
-- Panel bursts (the original's effect object #0x24, `sub_80E2F56`):
-- one-shot effects going off one panel at a time, every `interval` ticks,
-- over a set of panels in a random order: the blasts a time bomb's
-- explosion (chips/timebom/countdown), a BlkBomb's (chips/blkbomb/bomb),
-- an ElemTrap's counterattack (chips/elemtrap/strike) and MstrCros's
-- finale (chips/mstrcros/navi) scatter over the field. Only drawn, but its
-- shuffle draws the simulation RNG.
--
-- Its spawner (`sub_80E2FE8`, which seven routines call) is exported:
-- `panel_bursts.spawn(spawner, region, interval, z, effect)`. What goes
-- off was a row of the original's `byte_80E2F48` by the first parameter
-- (an effect and a sound, which no row has); it is the spawner's argument
-- now.

local panels = require("../../lib/panels")

export type State = {
    -- The panels still to burst, packed x | y << 4 (the object's +0x60),
    -- and how many (the original's second parameter).
    targets: { number },
    count: number,
    -- What goes off on each, and its sound (none: silent).
    effect: Effect,
    sound: Sound?,
}

local panel_bursts = {}

-- The object's list holds 28 panels (its extra bytes); more would run
-- into its sprite.
local CAPACITY = 28

-- A hit region's panels count when they are on the field
-- (`object_checkPanelParameters` with bit 0x10000, set on every refreshed
-- panel).
local ON_FIELD = 0x10000

-- `sub_80E3038`: the panels of a hit region around (x, y), the offsets' x
-- toward `side`'s front, that are on the field.
local function region_panels(offsets: { { number } }, x: number, y: number, side: number): { number }
    local dir = panels.side_direction(side)
    local list: { number } = {}
    for _, d in offsets do
        local px: number, py: number = d[1] * dir + x, d[2] + y
        if field.check(px, py, ON_FIELD, 0) then
            list[#list + 1] = panels.pack(px, py)
        end
    end
    return list
end

-- `sub_80E2F8C`: when the timer runs out, the next panel (last to first)
-- bursts; with none left, gone.
local function tick(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.timer = me.timer2
    local s = me.state :: State
    local n = s.count - 1
    s.count = int.u8(n)
    if n < 0 then
        me:set_lifecycle("destroy")
        return
    end
    local p = s.targets[n + 1]
    local x, y = field.center(panels.x(p), panels.y(p))
    battle.effect(Vec3.new(x, y, me.pos.z), s.effect)
    local sound = s.sound
    if sound then
        battle.play_sound(sound)
    end
end

-- `sub_80E2F74`: shuffle the panels (as many swaps as panels), then burst.
local function init(me: Object)
    local s = me.state :: State
    local n = s.count
    local list = {}
    for i = 1, n do
        list[i] = s.targets[i]
    end
    panels.shuffle(list, n, n)
    for i = 1, n do
        s.targets[i] = list[i]
    end
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80E2F56`.
local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

panel_bursts.kind = define.kind {
    id = "panel-bursts",
    pool = "effect",
    state = { targets = "u8[28]", count = "u8", effect = "effect", sound = "sound" },
    update = update,
}

-- `sub_80E2FE8`: bursts of `effect` (with `sound`, if any) over the panels
-- of `region` (a whole-field region's panels; else its panels around the
-- panel (`x`, `y`), facing `spawner`'s side's way), one every `interval`
-- ticks at height `z`. (Its X is `interval` and its Y the spawner's
-- registers, which nothing reads.)
function panel_bursts.spawn(
    spawner: Object,
    region: Region,
    interval: number,
    z: number,
    effect: Effect,
    sound: Sound?,
    x: number?,
    y: number?
): Object?
    local o = battle.spawn(panel_bursts.kind, Vec3.new(interval, 0, z))
    if not o then
        return nil
    end
    o.timer2 = interval
    local list
    local cond = region.field
    if cond then
        o.panel_x, o.panel_y = 0, 0
        list = panels.filtered_except(o, cond.require, cond.forbid)
    else
        list = region_panels(region.panels or {}, x or 0, y or 0, spawner.alliance)
    end
    if #list > CAPACITY then
        error(string.format("sub_80E2FE8: %d panels overrun the burst object's list", #list))
    end
    local s = o.state :: State
    for i, p in list do
        s.targets[i] = p
    end
    s.count = #list
    s.effect, s.sound = effect, sound
    return o
end

return panel_bursts
