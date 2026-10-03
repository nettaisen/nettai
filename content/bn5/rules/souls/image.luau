--!strict
-- The soul's image (actor object 0x2A, 0x080C0394; spawned by 0x080C0524, BN6's `sub_80C02A6`, which spawns its
-- own kind 0x28 for battle mode 9): the navi MegaMan unites with comes in on a spiral around him for 22 ticks
-- (tinted), blinks for 30 and fades out over 8. Its sprite is the soul's navi's (category 8 by the soul's number).
-- It plays the soul's start hook as it comes (0x0800ED0E with 1) and its end hook as it goes (0x0800EEBE with 1):
-- none for ProtoSoul.

local SINE: { number } = require("../math").sine

local image = {}

export type State = {
    -- The soul's number (Param1).
    soul: number,
}

-- The souls' navis' sprites, by soul number.
local SPRITES: { [number]: SpriteAsset } = {
    [1] = asset.sprite("bn5:protoman"),
}
local SOUND = asset.sound("bn5:cross-merge")
-- The tint while it spirals in.
local TINT = 0x2108

local function sine(i: number): number
    return SINE[i + 1]
end

-- 0x080C04F2: around its owner at `radius` pixels and angle `angle` (256 steps a turn) across and in height, two
-- pixels up and on.
local function place(me: Object, radius: number, angle: number)
    local owner = me.related1 :: Object
    local dx = int.shl(sine(angle + 64) * radius, 8)
    local dz = int.shl(sine(angle) * radius, 8)
    local p = owner.pos
    me.pos = Vec3.new(p.x + dx, p.y + 0x2_0000, p.z + dz + 0x2_0000)
end

-- 0x080C042C: 22 ticks spiraling in.
local function spiral(me: Object)
    if me.phase_init == 0 then
        me.timer = 22
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) < 0 then
        me.action, me.phase, me.phase_init = 4, 0, 0
        return
    end
    local t = me.timer
    place(me, t * 4, bit32.band(t * 112, 0xFF))
    me.sprite.color_shader = TINT
end

-- 0x080C046C: 30 ticks blinking, two ticks of four.
local function blink(me: Object)
    if me.phase_init == 0 then
        me.timer = 30
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) < 0 then
        me.visible = true
        me.action, me.phase, me.phase_init = 8, 0, 0
        return
    end
    me.visible = not bit32.btest(me.timer, 2)
end

-- 0x080C04AE: back on MegaMan, 8 ticks fading out; then it goes.
local function fade(me: Object)
    if me.phase_init == 0 then
        place(me, 0, 0)
        me.timer = 8
        me.phase_init = 4
    end
    me.timer -= 1
    if int.i16(me.timer) < 0 then
        me.sprite.alpha = nil
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    me.sprite.alpha = me.timer * 2
end

-- 0x080C040E
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        spiral(me)
    elseif a == 4 then
        blink(me)
    elseif a == 8 then
        fade(me)
    else
        error(string.format("the soul's image's action %#x reads past its table", a))
    end
end

-- 0x080C03B4
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    local look = SPRITES[s.soul]
    if not look then
        error(string.format("soul %d has no image's sprite (0x080C03B4)", s.soul))
    end
    sprite:load(look)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:update_sprite()
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

image.kind = define.kind {
    id = "bn5:soul-image",
    pool = "actor",
    state = { soul = "u8" },
    -- 0x080C0394
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

-- 0x080C0524: the image of soul `soul` for `owner`, on its side and facing.
function image.spawn(owner: Object, soul: number): Object?
    local o = battle.spawn(image.kind, owner.pos)
    if o then
        (o.state :: State).soul = soul
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
    end
    return o
end

return image
