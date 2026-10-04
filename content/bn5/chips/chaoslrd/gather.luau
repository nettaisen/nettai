--!strict
-- A flame of ChaosLrd's gathering (BN5's effect object #0x4B, 0x080E5E6C; BN5's own, its spawner where BN6 has
-- AirRaid's): set off from a point by an offset (sprite 08-14, animation 21, in one of two palettes), it circles in
-- on the point over 30 ticks, its circle shrinking (a quarter step a tick for the first ten, then whole steps,
-- doubled for the last five) and its turning sped up at random on its 11th, 21st and 26th tick (the battle RNG's
-- second stream). It runs while dimmed.

local trajectory = require("../../lib/trajectory")

export type State = {
    -- Its point's X (+0x68), its radius and its radius' step (+0x60, +0x64: 16.16).
    cx: number,
    radius: number,
    step: number,
    -- Its angle (Param3, 256 a turn) and its turn a tick (Param4, a byte: toward the side's back at first).
    angle: number,
    turn: number,
    -- Ticks left (Param1).
    left: number,
    -- 27 pixels up (+0x74, its Param4 when it came).
    lifted: boolean,
    -- Where it sets off from (+0x70: x toward its side's front in the high half, up in the low, in pixels), and its
    -- palette (Param2).
    offset: number,
    palette: number,
}

local gather = {}

local SPRITE = asset.sprite("chaos-beast")
local ANIM = 21
-- How long it circles in, and the ticks its turning changes at (ticks left).
local TICKS = 0x1E
local LIFT = 27

-- `object_getAllianceDirection`: +1 for side 0, -1 for side 1.
local function side_direction(me: Object): number
    return 1 - 2 * bit32.band(me.alliance, 1)
end

-- A halfword add to a 16.16 word's whole part.
local function whole_add(v: number, n: number): number
    local hi = bit32.band(bit32.rshift(v, 16) + n, 0xFFFF)
    return int.i32(bit32.bor(bit32.band(v, 0xFFFF), bit32.lshift(hi, 16)))
end

-- 0x080E5FE6: on its circle at its angle around its point (its Y kept), 27 pixels up if lifted.
local function place(me: Object)
    local s = me.state :: State
    local x = int.i32(int.asr(int.mul32(trajectory.sine(s.angle + 64), s.radius), 8) + s.cx)
    local z = int.i32(-int.asr(int.mul32(trajectory.sine(s.angle), s.radius), 8))
    if s.lifted then
        z = whole_add(z, LIFT)
    end
    me.pos = Vec3.new(x, me.pos.y, z)
end

-- Action 0 (0x080E5F30): a tick in.
local function circle(me: Object)
    local s = me.state :: State
    local back = -side_direction(me)
    local n = s.left
    if n >= 20 then
        if n == 20 then
            s.turn = int.u8(((bit32.band(battle.rng_positive(), 7) + 3) * back))
        end
        s.radius = int.i32(s.radius - int.asr(s.step, 2))
    else
        if n == 10 then
            s.turn = int.u8(s.turn + (bit32.band(battle.rng_positive(), 7) + 10) * back)
        elseif n == 5 then
            s.turn = int.u8(s.turn + (bit32.band(battle.rng_positive(), 7) + 16) * back)
            s.step = int.shl(s.step, 1)
        end
        s.radius = int.i32(s.radius - s.step)
    end
    s.angle = int.u8(s.angle - s.turn)
    place(me)
    s.left = int.u8(n - 1)
    if s.left == 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080E5E90: its sprite (no shadow, its palette), off its point by its offset; its radius (the offset's length) and step (a thirtieth of it), its
-- angle toward the offset, its turn toward its side's back; shown. (No update on its first tick.)
local function init(me: Object)
    local s = me.state :: State
    local offset = s.offset
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- (Its animation as requested, not yet loaded.)
    me.anim = ANIM
    sprite:set_animation(ANIM)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    sprite.palette = s.palette
    local p = me.pos
    s.cx = p.x
    local xo = int.i32(bit32.lshift(bit32.rshift(offset, 16), 16))
    local zo = int.i32(bit32.lshift(offset, 16))
    local z = int.i32(-zo)
    local ax, az = int.asr(xo, 16), int.asr(z, 16)
    s.radius = int.shl(trajectory.sqrt(int.u32(ax * ax + az * az)), 16)
    s.step = trajectory.div(s.radius, s.left)
    s.angle = trajectory.angle(xo, zo)
    s.turn = int.u8(-side_direction(me))
    if s.lifted then
        z = whole_add(z, LIFT)
    end
    me.pos = Vec3.new(int.i32(p.x + xo), p.y, z)
    me.visible = true
    me:set_lifecycle("update")
end

gather.kind = define.kind {
    id = "chaoslrd/gather",
    pool = "effect",
    state = {
        cx = "i32",
        radius = "i32",
        step = "i32",
        angle = "u8",
        turn = "u8",
        left = "u8",
        lifted = "bool",
        offset = "u32",
        palette = "u8",
    },
    -- 0x080E5E6C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            if me.action ~= 0 then
                error(string.format("ChaosLrd's gathering: action %#x reads past its table (0x080E5F2C)", me.action))
            end
            circle(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080E5FD2: a flame of `owner`'s at `pos` (its point), off it by `dx` (toward `owner`'s front) and `dz` pixels, in
-- `palette`; 27 pixels up; running while dimmed.
function gather.spawn(owner: Object, pos: Vec3, dx: number, dz: number, palette: number): Object?
    local o = battle.spawn(gather.kind, pos)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.offset = bit32.bor(bit32.lshift(bit32.band(dx * owner.facing, 0xFFFF), 16), bit32.band(dz, 0xFFFF))
        s.palette = palette
        s.left = TICKS
        s.lifted = true
        o.run_while_dimmed = true
    end
    return o
end

return gather
