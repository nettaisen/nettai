--!strict
-- Sand thrown up by SandWrm's worm (the original's attack object #0xCC,
-- `sub_80DEE60`): hidden until its delay has run, it arcs (22 pixels high)
-- over one panel toward the worm's user, then goes. Its level is the
-- worm's (its palette and arc length), its look one of two (animations
-- from 9 or 5).

local trajectory = require("../../lib/trajectory")

local spray = {}

export type State = {
    level: number,
    look: number,
    -- ExtraVars: the panel it lands on, the last animation set, its ticks
    -- (the delay, then ticks into the arc), the arc's length, the angle's
    -- step and the angle (as the worm's).
    target_x: number,
    target_y: number,
    last_anim: number,
    ticks: number,
    length: number,
    step: number,
    angle: number,
}

local SPRITE = asset.sprite("sand-worm")
-- The arc's length by level (`byte_80DEA50`), its height (`dword_80DF0A0`,
-- in units of the sine table), and the first animation by look
-- (`dword_80DEEE4`, `byte_80DEFC4`).
local LENGTH = { [0] = 0x10, 0x10, 0x10 }
local HEIGHT = 0x16
local FIRST_ANIM = { [0] = 9, 5 }

-- The actions.
local WAIT, ARC, DONE = 0, 4, 8

-- The look's first animation.
local function first_anim(s: State): number
    local a = FIRST_ANIM[s.look]
    if a == nil then
        error(string.format("sand look %d reads past dword_80DEEE4", s.look))
    end
    return a
end

-- `sub_80DF016`: turned toward the user; the arc to the panel behind.
local function aim(me: Object, s: State)
    me.flip = 1
    s.target_x = me.panel_x + me.facing
    s.target_y = me.panel_y
    local n = LENGTH[s.level]
    if n == nil then
        error(string.format("sand level %d reads past byte_80DEA50", s.level))
    end
    -- sub_8114F6A
    local tx, ty = field.center(s.target_x, s.target_y)
    local p = me.pos
    me.vel = Vec3.new(int.tdiv(int.i32(tx - p.x), n), int.tdiv(int.i32(ty - p.y), n), int.tdiv(int.i32(0 - p.z), n))
    s.step = int.tdiv(0x8000, n)
    s.length = n
    s.angle = 0
    local a = first_anim(s)
    me.anim = a
    s.last_anim = a
end

-- `sub_80DEF1C`: the delay.
local function wait(me: Object, s: State)
    local left = s.ticks - 1
    s.ticks = left
    if left ~= 0 then
        return
    end
    me.visible = true
    me.action = ARC
    me.phase = 0
end

-- `sub_8114F52`: which quarter of the arc (0 to 3).
local function quarter(ticks: number, length: number): number
    local q = int.tdiv(ticks, int.tdiv(length, 4))
    return if q == 4 then 3 else q
end

-- `sub_80DEF3C`: along the arc.
local function arc(me: Object, s: State)
    local p = me.pos
    local sine = int.u16(trajectory.sine(bit32.rshift(s.angle, 8)))
    me.pos = Vec3.new(p.x + me.vel.x, p.y, bit32.lshift(sine * HEIGHT, 8))
    local anim = quarter(s.ticks, s.length) + first_anim(s)
    if anim ~= s.last_anim then
        me.anim = anim
        s.last_anim = anim
    end
    s.angle = s.angle + s.step
    s.ticks = s.ticks + 1
    if s.ticks == s.length then
        me.anim = 0
        local q = me.pos
        me.pos = Vec3.new(q.x, q.y, 0)
        me.action = DONE
        me.phase = 0
    end
    me:set_panel_from_coordinates()
    -- (object_updateCollisionPanels: it has no collision, and the game's
    -- writes through the null pointer land outside RAM.)
end

-- `sub_80DEEE8`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    local s = me.state :: State
    local a = me.action
    if a == WAIT then
        wait(me, s)
    elseif a == ARC then
        arc(me, s)
    elseif a == DONE then
        -- sub_80DEFC6
        me.visible = false
        me:set_lifecycle("destroy")
    else
        error(string.format("sand action %#x reads past its table", a))
    end
end

-- `sub_80DEE8C`.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = false
    local a = first_anim(s)
    me.anim, me.anim_loaded = a, a
    sprite:set_animation(a)
    sprite.palette = s.level
    aim(me, s)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

spray.kind = define.kind {
    id = "sandwrm/spray",
    pool = "attack",
    state = {
        level = "u8",
        look = "u8",
        target_x = "u8",
        target_y = "u8",
        last_anim = "u8",
        ticks = "u16",
        length = "u16",
        step = "u16",
        angle = "u16",
    },
    -- `sub_80DEE60`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80DEFD6`: sand of look `look` from `worm` (of `level`) on its panel,
-- after `delay` ticks. (It spawns at the registers' values: the worm's Y
-- and the delay.)
function spray.spawn(worm: Object, level: number, look: number, delay: number): Object?
    local o = battle.spawn(spray.kind, Vec3.new(worm.panel_y, delay, 0))
    if o then
        o.related1 = worm
        o.panel_x, o.panel_y = worm.panel_x, worm.panel_y
        local s = o.state :: State
        s.level, s.look, s.ticks = level, look, delay
        o.alliance, o.flip = worm.alliance, worm.flip
        local p = worm.pos
        o.pos = Vec3.new(p.x, p.y - 1, p.z)
    end
    return o
end

return spray
