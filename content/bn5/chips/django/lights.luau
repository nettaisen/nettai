--!strict
-- Django's lights (BN5's effect object #0x7D, 0x080E9968; BN5's own): over his target's panel, three lights (its
-- children: the same kind, Param1 1) circle the coffin, 35 pixels up, a third of a turn apart, 8 steps of 256 a
-- tick, on an ellipse 45 pixels across and 28 deep; the one that leads them stays hidden. They go when Django ends
-- them (`lights.remove`), brighter with Otenko's blessing (palette 3).
--
-- (The children's panel bytes are register garbage: the leader's header flags and the last child's address, which
-- nothing reads; compat's `scratch_panel`.)

local trajectory = require("../../lib/trajectory")

local lights = {}

export type State = {
    -- A light (its Param1 1), or the leader (0).
    light: boolean,
    -- Otenko blesses Django's side (its Param2: palette 3 for 1).
    blessed: boolean,
    -- It is ended (+0x40).
    done: boolean,
    -- The leader's lights (+0x60, eight slots it walks; it fills three).
    children: { Object? },
}

local SPRITE = asset.sprite("django-sun")
-- The lights' start angles (0x080E9A60).
local STARTS = { 0x00, 0x55, 0xAA }
local sine = trajectory.sine

local spawn: (owner: Object, x: number, y: number, light: boolean, blessed: boolean) -> Object?

-- 0x080E9A80: round the leader, on its ellipse, at the timer's angle (turning against its facing).
local function orbit(me: Object)
    local owner = me.related1 :: Object
    local a = bit32.band(me.timer, 0xFF)
    local p, q = me.pos, owner.pos
    me.pos = Vec3.new(
        int.i32(q.x + int.mul32(int.shl(int.mul32(sine(a + 64), 0x2D), 8), -me.facing)),
        int.i32(q.y + int.shl(int.mul32(sine(a), 0x1C), 8)),
        p.z
    )
end

-- 0x080E99CC: the angle on; ended, it goes (the leader ending its lights); a light keeps to its ellipse.
local function tick(me: Object)
    local s = me.state :: State
    if me.phase ~= 0 then
        error(string.format("Django's lights phase %#x reads past its table (0x080E99E8)", me.phase))
    end
    -- 0x080E99EC.
    me.timer = int.u16(me.timer + 8)
    if s.done then
        me.visible = false
        if not s.light then
            -- 0x080E9A64.
            for i = 1, 8 do
                local c = s.children[i]
                if c then
                    (c.state :: State).done = true
                end
            end
        end
        me:set_lifecycle("destroy")
    end
    if s.light then
        orbit(me)
    end
end

-- 0x080E998C: on its panel (animation 2, loaded with the sprite update), drawn with its sprite; the leader hidden,
-- 35 pixels up, with its three lights; a light 24 up.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 2, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me:set_coordinates_from_panel()
    local z = 0x18_0000
    if not s.light then
        me.visible = false
        -- 0x080E9A32.
        for i, start in STARTS do
            local c = spawn(me, 0, 0, true, s.blessed)
            s.children[i] = c
            if c then
                c.timer = start
            end
        end
        z = 0x23_0000
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, z)
    sprite.palette = if s.blessed then 3 else 0
    me:set_lifecycle("update")
    tick(me)
end

lights.kind = define.kind {
    id = "django/lights",
    pool = "effect",
    state = { light = "bool", blessed = "bool", done = "bool", children = "object[8]" },
    -- 0x080E9968.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080E9A10: a leader on (x, y) for `owner` (Django), or one of a leader's lights.
spawn = function(owner: Object, x: number, y: number, light: boolean, blessed: boolean): Object?
    local o = battle.spawn(lights.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: State
        s.light = light
        s.blessed = blessed
    end
    return o
end

-- Django's lights over (x, y).
function lights.spawn(django: Object, x: number, y: number, blessed: boolean): Object?
    return spawn(django, x, y, false, blessed)
end

-- 0x080E9A2C: they are ended.
function lights.remove(me: Object)
    (me.state :: State).done = true
end

return lights
