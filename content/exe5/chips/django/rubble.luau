--!strict
-- Rubble from Django's sun's landing (EXE5's effect object #0x84, 0x080EA000; EXE5's own): a piece that drops from
-- where it is spawned, puffing as it starts, falling faster each tick (half a pixel more), then lying blinking for
-- 20 ticks. (Its Param1 picks its look: Django's 0, animation 0 and the puff effect 2; the second, animation 1 and
-- effect 1, no spawner asks for.)

local rubble = {}

local SPRITE = asset.sprite("django-rubble")
-- The puff it starts with: effect 2 (the small puff's animation 1).
local PUFF = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 }
local GRAVITY = 0x8000

-- Phase 0 (0x080EA074): falling; on the ground, 20 ticks.
local function fall(me: Object)
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z - GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local z = int.i32(p.z + vz)
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), z)
    if z <= 0 then
        me.timer = 0x14
        me.phase, me.phase_init = 4, 0
    end
end

-- Phase 4 (0x080EA09E): blinking (hidden on the timer's bit 2); then gone.
local function lie(me: Object)
    me.visible = bit32.band(me.timer, 4) == 0
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

-- 0x080EA05A.
local function tick(me: Object)
    if me.phase == 0 then
        fall(me)
    elseif me.phase == 4 then
        lie(me)
    else
        error(string.format("Django's rubble phase %#x reads past its table (0x080EA06C)", me.phase))
    end
end

-- 0x080EA024: its look (animation 0, loaded with the sprite update), its shadow, and a puff where it is.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    battle.effect(me.pos, PUFF, 0, 0, 0)
    me:set_lifecycle("update")
    tick(me)
end

rubble.kind = new.kind {
    id = "django/rubble",
    pool = "effect",
    -- 0x080EA000.
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

-- 0x080EA0CA: a piece of `owner`'s at `pos`.
function rubble.spawn(owner: Object, pos: Vec3): Object?
    local o = battle.spawn(rubble.kind, pos)
    if o then
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return rubble
