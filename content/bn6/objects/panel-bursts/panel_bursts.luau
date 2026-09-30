--!strict
-- Panel bursts (effect object #0x24, `sub_80E2F56`): one-shot effects
-- (effect #0) going off one panel at a time, every `interval` ticks, over a
-- set of panels in a random order: the blasts a time bomb's explosion
-- (objects/countdown-bomb) and an ElemTrap's counterattack
-- (objects/elem-trap-strike) scatter over the field. Only drawn, but its
-- shuffle draws the simulation RNG.
--
-- Its spawner (`sub_80E2FE8`, which seven routines call) is exported:
-- `panel_bursts.spawn(spawner, region, interval, z, params)`.

local panels = require("../../lib/panels")

export type State = {
    -- The panels still to burst, packed x | y << 4 (the object's +0x60;
    -- Param2 counts them).
    targets: { number },
}

local panel_bursts = {
    state = { targets = "u8[28]" },
}

-- The object's list holds 28 panels (its extra bytes); more would run
-- into its sprite.
local CAPACITY = 28

-- Region numbers from 0x80 cover the panels meeting a whole-field
-- condition (`data.rules.field_regions`); below, a hit region's offsets
-- (`data.regions`) from the anchor panel.
local WHOLE_FIELD = 0x80
-- A hit region's panels count when they are on the field
-- (`object_checkPanelParameters` with bit 0x10000, set on every refreshed
-- panel).
local ON_FIELD = 0x10000

-- `byte_80E2F48`: what goes off, by Param1: the effect's look and a sound
-- (0 = none).
local LOOKS: { [number]: { effect: number, sound: number } } = {
    [0] = { effect = 0x1F, sound = 0 },
    [1] = { effect = 0x00, sound = 0 },
    [2] = { effect = 0x45, sound = 0 },
    [3] = { effect = 0x22, sound = 0 },
    [4] = { effect = 0x0B, sound = 0 },
    [5] = { effect = 0x6B, sound = 0 },
    [6] = { effect = 0x24, sound = 0 },
}

-- `sub_80E3038`: the panels of hit region `region` around (x, y), the
-- offsets' x toward `side`'s front, that are on the field.
local function region_panels(region: number, x: number, y: number, side: number): { number }
    local offsets = data.regions[region]
    if not offsets then
        error(string.format("sub_80E3038: hit region %#x reads past PanelOffsetListsPointerTable", region))
    end
    -- `object_getAllianceDirection`.
    local dir = 1 - 2 * side
    local list: { number } = {}
    for _, d in offsets do
        local px: number, py: number = d[1] * dir + x, d[2] + y
        if field.check(px, py, ON_FIELD, 0) then
            list[#list + 1] = panels.pack(px, py)
        end
    end
    return list
end

-- `sub_80E2FE8`: bursts over the panels of `region` (from 0x80, the whole
-- field's panels meeting condition `region - 0x80`; else hit region
-- `region` around the panel (Param3, Param4) of `params`, facing
-- `spawner`'s side's way), one every `interval` ticks at height `z`, with
-- the look `params[1]` (`LOOKS`). (Its X is `interval` and its Y the
-- spawner's registers, which nothing reads.)
function panel_bursts.spawn(spawner: Object, region: number, interval: number, z: number, params: { number }): Object?
    local o = battle.spawn_kind("panel-bursts", Vec3.new(interval, 0, z), params)
    if not o then
        return nil
    end
    o.timer2 = interval
    local list
    if bit32.btest(region, WHOLE_FIELD) then
        local i = bit32.band(region, 0x7F)
        local cond = data.rules.field_regions[i]
        if not cond then
            error(string.format("sub_80E2FE8: whole-field region %#x reads past byte_8019C34", 0x80 + i))
        end
        o.panel_x, o.panel_y = 0, 0
        list = panels.filtered_except(o, cond.require, cond.forbid)
    else
        list = region_panels(region, o:param(3), o:param(4), spawner.alliance)
    end
    if #list > CAPACITY then
        error(string.format("sub_80E2FE8: %d panels overrun the burst object's list", #list))
    end
    local s = o.state :: State
    for i, p in list do
        s.targets[i] = p
    end
    o:set_param(2, #list)
    return o
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
    local n = me:param(2) - 1
    me:set_param(2, int.u8(n))
    if n < 0 then
        me:set_lifecycle("destroy")
        return
    end
    local look = LOOKS[me:param(1)]
    if not look then
        error(string.format("panel bursts look %d reads past byte_80E2F48", me:param(1)))
    end
    local s = me.state :: State
    local p = s.targets[n + 1]
    local x, y = field.center(panels.x(p), panels.y(p))
    battle.effect(Vec3.new(x, y, me.pos.z), look.effect)
    if look.sound ~= 0 then
        battle.play_sound(look.sound)
    end
end

-- `sub_80E2F74`: shuffle the panels (as many swaps as panels), then burst.
local function init(me: Object)
    local s = me.state :: State
    local n = me:param(2)
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
function panel_bursts.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
end

return panel_bursts
