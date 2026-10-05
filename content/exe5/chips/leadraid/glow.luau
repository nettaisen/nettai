--!strict
-- LeadRaid's charge glow (EXE5's effect object #0x85, 0x080EA0EC; EXE5's own, which only LeadRaid's leaders spawn): on
-- its leader (sprite 14-08), building for 20 ticks with the buster's charging sound, then charged (animation 2) with
-- its sound, until its leader ends it (0x080EA1B0).

export type State = {
    -- Its leader's attach point 0, by his NameID (0x08015148: the x toward his facing, and the z). The code adds the
    -- pixels to his 16.16 position as they are, a fraction of a pixel.
    dx: number,
    dz: number,
}

local glow = {}

local SPRITE = asset.sprite("charge-glow")
local SOUND_CHARGE, SOUND_CHARGED = asset.sound("buster-charge"), asset.sound("buster-charged")
-- How long it builds (to 0), and its charged animation.
local BUILD, CHARGED = 0x14, 2

-- 0x080EA136: on its leader, at his attach point 0 (its x mirrored by his side and flip, `object_getFlipDirection`).
local function follow(me: Object)
    local s = me.state :: State
    local leader = me.related1 :: Object
    local dx = if bit32.bxor(leader.alliance, leader.flip) == 0 then s.dx else -s.dx
    local p = leader.pos
    me.pos = Vec3.new(int.i32(p.x + dx), p.y, int.i32(p.z + s.dz))
end

-- 0x080EA12A: its phase (0x080EA168: building, then charged; then nothing, 0x080EA196); on its leader.
local function tick(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            battle.play_sound(SOUND_CHARGE)
            me.timer = BUILD
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left <= 0 then
            me.anim = CHARGED
            battle.play_sound(SOUND_CHARGED)
            me.phase, me.phase_init = 4, 0
        end
    elseif me.phase ~= 4 then
        error(string.format("LeadRaid's glow: phase %#x reads past its table (0x080EA160)", me.phase))
    end
    follow(me)
end

-- 0x080EA110: its sprite (`sub_801BE2A`'s word 0x00001408: 14-08, animation 0, no shadow), shown; its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

glow.kind = define.kind {
    id = "leadraid/glow",
    pool = "effect",
    state = { dx = "i32", dz = "i32" },
    -- 0x080EA0EC.
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

-- 0x080EA198: a glow on `leader`, his side's and facing, at his attach point 0 (`dx` toward his facing, `dz` up, in
-- pixels). (Its position is the spawner's registers until its first update places it.)
function glow.spawn(leader: Object, dx: number, dz: number): Object?
    local o = battle.spawn(glow.kind)
    if o then
        o.alliance, o.flip = leader.alliance, leader.flip
        o.related1 = leader
        local s = o.state :: State
        s.dx, s.dz = dx, dz
    end
    return o
end

-- 0x080EA1B0: its leader ends it.
function glow.stop(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return glow
