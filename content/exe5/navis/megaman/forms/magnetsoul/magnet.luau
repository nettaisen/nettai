--!strict
-- MagnetSoul's magnet field (EXE5's attack object 0x75, 0x080D4B60: EXE5's own, where EXE6's is AirRaid's plane): an
-- unseen attack on the panel at an offset from its owner, following him, for 30 ticks: an attack against navis
-- (hit modifier 3, its element's spark), which paralyzes for 90 ticks with no hit modifier when its parameter says.
-- It goes once it hits something, when its owner lets go of it (his first related object cleared), or when the
-- battle ends.

local collision = require("../../../../rules/collision")
local sparks = require("../../../../lib/sparks")
local status = require("../../../../rules/status")

local magnet = {}

export type State = {
    -- Param1: it paralyzes.
    paralyzes: boolean,
    -- +0x40 and +0x44: its offset from its owner's panel.
    dx: number,
    dy: number,
}

local HIT_MOD = 3
local LIFETIME = 30

-- Gone: no region, and freed at the next update.
local function leave(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D4BF4: 30 ticks on its owner's panel and offset.
local function follow(me: Object)
    if me.phase_init == 0 then
        me.timer = LIFETIME
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        leave(me)
        return
    end
    local s = me.state :: State
    local owner = me.related1 :: Object
    me.panel_x, me.panel_y = owner.panel_x + s.dx, owner.panel_y + s.dy
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- 0x080D4BBE: its hits resolve and spark; while the battle runs, nothing was hit and its owner holds it, it follows.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local owner = me.related1 :: Object
    if battle.over() or c.hit_flags ~= 0 or owner.related1 == nil then
        leave(me)
        return
    end
    follow(me)
    c:present()
end

-- 0x080D4B80: on its panel, an attack against navis with its element's spark; then its first update.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.of_element(me.element))
    if (me.state :: State).paralyzes then
        c.status_base = status.paralyze_90
        c.hit_mod_base = 0
    end
    me:set_lifecycle("update")
    tick(me)
end

magnet.kind = define.kind {
    id = "magnetsoul/magnet",
    pool = "attack",
    state = { paralyzes = "bool", dx = "i32", dy = "i32" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080D4C32: the magnet `owner` holds (his first related object), at (dx, dy) from his panel, of `element` and
-- damage word `damage`, paralyzing or not. (Its spawn position is the spawner's registers, and it has no panel
-- until its first update follows its owner.)
function magnet.spawn(
    owner: Object,
    dx: number,
    dy: number,
    element: number,
    paralyzes: boolean,
    damage: number
): Object?
    local o = battle.spawn(magnet.kind, Vec3.new(dy, element, 0))
    if o then
        local s = o.state :: State
        s.paralyzes, s.dx, s.dy = paralyzes, dx, dy
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.related1 = owner
        owner.related1 = o
    end
    return o
end

return magnet
