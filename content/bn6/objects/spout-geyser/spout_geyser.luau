--!strict
-- SpoutMan's geyser (attack object #0x17, `sub_80C6DCC`): from his panel it
-- picks the column it erupts in (the first one ahead holding something of
-- the other side's, the target column Param3, or the field's edge), marks
-- it and the trail to it (objects/spout-mark), highlights them, and for
-- Param2 ticks hits them every 30 ticks, from the first: the column's
-- three panels and the trail, with hit regions that last 30 ticks. With
-- Param1 4 (SpoutMan's) it acts and steps its sprite while dimmed; with
-- another (his navi AI's) it waits a dimming out and goes as soon as its
-- owner isn't in action 0xB. See docs/engine/chips.md §3.6.11.

local spout_mark = require("../spout-mark/spout_mark")

local spout_geyser = {}

local SPRITE = "10-20"

-- The owner's action a navi AI's geyser lasts through.
local OWNER_ACTION = 0xB

-- What stops the search for its column, by side (`byte_80C6F48`): a panel
-- flags word any bit of which will do.
local STOPS = { [0] = 0x0580_0000, 0x0A80_0000 }

-- Its marks' Param3 by its Param1 (`byte_80C7028`).
local MARKS_WHILE_DIMMED = { [0] = 0, 0, 0, 0, 1, 0, 0, 0 }

-- How often it hits, and how long each hit region lasts.
local HIT_TICKS = 0x1E

-- `sub_80C6EE0`: a geyser from (x, y) for `owner` with `element`, the
-- damage word and `params` {Param1, ticks, target column}. `z` is the
-- spawner's register that becomes its Z, whose fraction it keeps.
function spout_geyser.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("spout-geyser", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C6F08`: the column it erupts in: the first of the five ahead
-- holding something of the other side's, or the target column, or the
-- field's edge (the fifth if none).
local function column(me: Object, target: number): number
    local stops = STOPS[me.alliance]
    local x = me.panel_x
    for i = 1, 5 do
        x = me.panel_x + me.facing * i
        if bit32.band(field.flags(x, me.panel_y), stops) ~= 0 or x == target or x <= 1 or x >= 6 then
            return x
        end
    end
    return x
end

-- The trail: the columns ahead of its panel, up to four, before `stop`.
local function each_trail(me: Object, stop: number, f: (x: number) -> ())
    for i = 1, 4 do
        local x = me.panel_x + me.facing * i
        if x == stop then
            return
        end
        f(x)
    end
end

-- `sub_80C6FC8`: the marks: the column's (Param1 1) and the trail's.
local function mark(me: Object, ticks: number, stop: number)
    local marks_param3 = MARKS_WHILE_DIMMED[me:param(1)]
    if marks_param3 == nil then
        error(string.format("SpoutMan's geyser %d reads past byte_80C7028", me:param(1)))
    end
    local owner = me.related1
    -- r3 (1, then 0) becomes each mark's Z.
    spout_mark.spawn(me, owner, stop, me.panel_y, { 1, ticks, marks_param3 }, 1)
    each_trail(me, stop, function(x)
        spout_mark.spawn(me, owner, x, me.panel_y, { 0, ticks, marks_param3 }, 0)
    end)
end

-- `sub_80C7030`: highlight the column's panels and the trail.
local function highlight(me: Object, stop: number)
    for dy = -1, 1 do
        local y = me.panel_y + dy
        if field.valid(stop, y) then
            field.highlight(stop, y)
        end
    end
    each_trail(me, stop, function(x)
        field.highlight(x, me.panel_y)
    end)
end

-- One hit region on (x, y), running while dimmed (`sub_80C53A6`) and
-- lasting `ticks`.
local function hit(me: Object, x: number, y: number, ticks: number)
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = 1,
        hit_effect = 2,
        target = 5,
        self_type = 4,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
        box.timer = ticks
    end
end

-- `sub_80C6F50`: hit the column's panels and the trail.
local function hit_all(me: Object, ticks: number, stop: number)
    for dy = -1, 1 do
        local y = me.panel_y + dy
        if field.valid(stop, y) then
            hit(me, stop, y, ticks)
        end
    end
    each_trail(me, stop, function(x)
        hit(me, x, me.panel_y, ticks)
    end)
end

-- `sub_80C6E90`: pick the column (Param4) and mark it; then each tick
-- highlight it, and hit every 30 ticks from the first, for Param2 ticks.
local function erupt(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me:set_param(4, column(me, me:param(3)))
        me.timer = me:param(2)
        me.timer2 = 1
        mark(me, me.timer, me:param(4))
    end
    local stop = me:param(4)
    highlight(me, stop)
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    local wait = me.timer2 - 1
    me.timer2 = wait
    if bit32.band(wait, 0xFFFF) ~= 0 then
        return
    end
    me.timer2 = HIT_TICKS
    hit_all(me, HIT_TICKS, stop)
end

-- `sub_80C6E50`: gone once the battle is over (or, not SpoutMan's, its
-- owner is out of action 0xB); outside a dimming unless it is SpoutMan's.
local function tick(me: Object)
    local own = me:param(1) == 4
    if battle.over() or (not own and (me.related1 :: Object).action ~= OWNER_ACTION) then
        me:set_lifecycle("destroy")
        return
    end
    if not own and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("SpoutMan's geyser action %#x reads past its table (off_80C6E8C)", me.action))
    end
    erupt(me)
end

-- `sub_80C6DFC`: on its panel, a pixel up and down (whole-part halfword
-- stores), then its first update at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    local z = bit32.bor(bit32.band(p.z, 0xFFFF), 1 * 0x10000)
    me.pos = Vec3.new(p.x, p.y + 1 * 0x10000, z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C6DCC`.
function spout_geyser.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 4 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return spout_geyser
