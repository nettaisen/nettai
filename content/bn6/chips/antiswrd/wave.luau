--!strict
-- AntiSwrd's sword wave (the original's attack object #0x58,
-- `sub_80CF7F0`): a slash over a column that flies forward from the panel
-- ahead of its user, hitting each column it enters, until it leaves the
-- field or (unless `piercing`) hits something. Its sprite is drawn as it
-- was set up: it doesn't animate.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local wave = {}

export type State = {
    -- Its hit modifier (the original's first parameter).
    hit_mod: number,
    -- It flies on through what it hits, in palette 0xB (the original's
    -- second parameter; nothing sets it).
    piercing: boolean,
}

local SPRITE = asset.sprite("sword-slash")
-- Its palette by element (`byte_80CF8A8`).
local PALETTES: { [number]: number } = { [0] = 0, 1, 2, 3, 4, 0, 0, 0 }
local PIERCING_PALETTE = 0xB
-- Bodies, objects and attacks of either side, and neutral objects: what
-- ends it.
local STOPS = 0x3F80_0000
-- What it is (a slash) and hits (navis).
local IS, HITS = collision.slash, collision.hits_navis

-- `sub_80CF8F8`: on along the row; gone off the field.
local function fly(me: Object): boolean
    me.pos = Vec3.new(int.i32(me.pos.x + me.vel.x), me.pos.y, me.pos.z)
    me:set_panel_from_coordinates()
    me.collision.panel_x, me.collision.panel_y = me.panel_x, me.panel_y
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me.lifecycle = "destroy"
        return false
    end
    return true
end

-- `sub_80CF8B0`: resolve last tick's hits; at the battle's end or (unless
-- piercing) on a hit, gone; else fly, hitting again in a new column.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    local stop = battle.over()
    if not stop and bit32.btest(c.hit_flags, STOPS) then
        if (me.state :: State).piercing then
            c:set_region(nil)
        else
            stop = true
        end
    end
    if stop then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    local x, y = me.panel_x, me.panel_y
    fly(me)
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.wide)
    end
    c:present()
end

-- `sub_80CF810`: unless the battle is over: on its panel, 16 pixels up,
-- flying the way it faces, its region on the column. On at once.
local function init(me: Object)
    if battle.over() then
        me:free()
        return
    end
    local s = me.state :: State
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = if s.piercing then PIERCING_PALETTE else PALETTES[bit32.band(me.element, 0xF)] or 0
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0x10_0000)
    me.timer = 0
    me.vel = Vec3.new(int.i32(me.facing * me.vel.x), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, s.hit_mod)
    me.collision:set_region(regions.wide)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

wave.kind = define.kind {
    id = "antiswrd/wave",
    pool = "attack",
    state = { hit_mod = "u8", piercing = "bool" },
    -- `sub_80CF7F0`.
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

-- `sub_80CF91E`: a wave on (x, y) from `user` with `element`, `hit_mod`
-- and the damage word, of its side and facing, flying `speed` a tick.
-- (Its position is the spawner's registers until its init places it.)
function wave.spawn(user: Object, x: number, y: number, element: number, hit_mod: number, damage: number, speed: number): Object?
    local o = battle.spawn(wave.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = user.alliance, user.flip
    o.related1 = user
    ;(o.state :: State).hit_mod = hit_mod
    o.vel = Vec3.new(speed, 0, 0)
    return o
end

return wave
