--!strict
-- SpoutMan's geyser (the original's attack object #0x17, `sub_80C6DCC`):
-- from his panel it picks the column it erupts in (the first one ahead
-- holding something of the other side's, its target column, or the
-- field's edge), marks it and the trail to it (./mark), highlights them,
-- and for its ticks hits them every 30 ticks, from the first: the column's
-- three panels and the trail, with hit regions that last 30 ticks. The
-- navi chip's (the original's Param1 4) acts and steps its sprite while
-- dimmed; another (his own AI's) waits a dimming out and goes as soon as
-- its owner isn't in action 0xB. What it is spawned with is a `Geyser`
-- (types.d.luau). See docs/engine/chips.md §3.6.11.

local mark = require("./mark")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local geyser = {}

export type State = {
    -- It acts, and animates, while the battle is dimmed (Param1 4).
    while_dimmed: boolean,
    -- How long it erupts (Param2), the column its spawner aims it at
    -- (Param3), and the column it erupts in (Param4).
    ticks: number,
    target: number,
    column: number,
}

local SPRITE = asset.sprite("bn6:spout-geyser")

-- The owner's action a navi AI's geyser lasts through.
local OWNER_ACTION = 0xB

-- What stops the search for its column, by side (`byte_80C6F48`): a panel
-- flags word any bit of which will do.
local STOPS = { [0] = 0x0580_0000, 0x0A80_0000 }

-- (Its marks act while dimmed as it does: `byte_80C7028`, 1 for the navi
-- chip's Param1 alone.)

-- How often it hits, and how long each hit region lasts.
local HIT_TICKS = 0x1E

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

-- `sub_80C6FC8`: the marks: the column's and the trail's.
local function leave_marks(me: Object, ticks: number, stop: number)
    local while_dimmed = (me.state :: State).while_dimmed
    local owner = me.related1
    -- r3 (1, then 0) becomes each mark's Z.
    mark.spawn(me, owner, stop, me.panel_y, { column = true, ticks = ticks, while_dimmed = while_dimmed }, 1)
    each_trail(me, stop, function(x)
        mark.spawn(me, owner, x, me.panel_y, { ticks = ticks, while_dimmed = while_dimmed }, 0)
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
        region = regions.single,
        hit_effect = sparks.aqua,
        target = collision.hits_navis,
        self_type = collision.attack,
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

-- `sub_80C6E90`: pick the column and mark it; then each tick highlight
-- it, and hit every 30 ticks from the first, for its ticks.
local function erupt(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        s.column = column(me, s.target)
        me.timer = s.ticks
        me.timer2 = 1
        leave_marks(me, me.timer, s.column)
    end
    local stop = s.column
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

-- `sub_80C6E50`: gone once the battle is over (or, not the navi chip's,
-- its owner is out of action 0xB); outside a dimming unless it is the navi
-- chip's.
local function tick(me: Object)
    local own = (me.state :: State).while_dimmed
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

geyser.kind = define.kind {
    id = "bn6:spoutman/geyser",
    pool = "attack",
    state = { while_dimmed = "bool", ticks = "u8", target = "u8", column = "u8" },
    -- `sub_80C6DCC`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80C6EE0`: a geyser from (x, y) for `owner` with `element`, the
-- damage word and what it is. `z` is the spawner's register that becomes
-- its Z, whose fraction it keeps.
function geyser.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    what: Geyser,
    damage: number,
    z: number
): Object?
    local o = battle.spawn(geyser.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.while_dimmed = what.while_dimmed == true
        s.ticks = what.ticks
        s.target = what.target
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return geyser
