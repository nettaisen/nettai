--!strict
-- Chaos Unison's dark shade (actor object 0x2B, 0x080C0544; spawned by 0x080C0678 as the soul's image comes in, ../
-- soul-image): MegaMan's own sprite in its dark palette leaves him forward for 20 ticks, waits 30, and spirals back
-- into him over 20; then it goes. (BN6 has the same object, actor 0x2A's neighbor `sub_80C065C`, which nothing
-- spawns.) It runs while the battle is paused, as the change does.

local SINE: { number } = require("../../rules/math").sine

local shade = {}

export type State = {
    -- The object's word +0x40: how far it moves across each tick.
    speed: number,
}

-- MegaMan's base sprite (00-00, 0x0801BE2A with 0x01000000: animation 0, with a shadow it then drops).
local SPRITE = asset.sprite("bn5:megaman")
-- His dark palette.
local PALETTE = 1

local function sine(i: number): number
    return SINE[i + 1]
end

-- Two pixels up and on from where it stands.
local LIFT = 0x2_0000

-- Its phases, on the object's phase.
local LEAVE, WAIT, RETURN = 0, 4, 8

local function set_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- `object_getFrontDirection`: +1 facing right, -1 facing left.
local function front_direction(me: Object): number
    return if bit32.bxor(me.alliance, me.flip) == 0 then 1 else -1
end

-- 0x080C05C4: 20 ticks forward, two pixels a tick.
local function leave(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.timer = 20
        s.speed = front_direction(me) * 2 * 0x1_0000
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) <= 0 then
        s.speed = 0
        set_phase(me, WAIT)
    end
    me.pos = Vec3.new(me.pos.x + s.speed, me.pos.y, me.pos.z)
end

-- 0x080C05F8: 30 ticks.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 30
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) <= 0 then
        set_phase(me, RETURN)
    end
end

-- 0x080C0646: around its owner at `radius` pixels and angle `angle` (256 steps a turn) across and in height, two
-- pixels up and on.
local function place(me: Object, radius: number, angle: number)
    local owner = me.related1 :: Object
    local dx = int.shl(sine(angle + 64) * radius, 8)
    local dz = int.shl(sine(angle) * radius, 8)
    local p = owner.pos
    me.pos = Vec3.new(p.x + dx, p.y + LIFT, p.z + dz + LIFT)
end

-- 0x080C0616: 20 ticks spiraling back in; then it goes.
local function come_back(me: Object)
    if me.phase_init == 0 then
        me.timer = 20
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) < 0 then
        me:set_lifecycle("destroy")
        return
    end
    local t = me.timer
    place(me, t * 2, bit32.band(t * 112, 0xFF))
end

-- 0x080C05A4
local function tick(me: Object)
    local p = me.phase
    if p == LEAVE then
        leave(me)
    elseif p == WAIT then
        wait(me)
    elseif p == RETURN then
        come_back(me)
    else
        error(string.format("the chaos shade's phase %#x reads past its table (0x080C05B8)", p))
    end
end

-- 0x080C0568
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    sprite.palette = PALETTE
    local p = (me.related1 :: Object).pos
    me.pos = Vec3.new(p.x, p.y + LIFT, p.z + LIFT)
    me:set_lifecycle("update")
    tick(me)
end

shade.kind = define.kind {
    id = "bn5:chaos-shade",
    pool = "actor",
    state = { speed = "i32" },
    -- 0x080C0544: the state, then its sprite steps.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080C0678: the shade for `owner`, on its side and facing, running while paused.
function shade.spawn(owner: Object): Object?
    local o = battle.spawn(shade.kind, owner.pos)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
        o.sprite:set_flip(bit32.bxor(owner.alliance, owner.flip))
    end
    return o
end

return shade
