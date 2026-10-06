--!strict
-- The soul's image (actor object 0x2A, 0x080C0394; spawned by 0x080C0524, EXE6's `sub_80C02A6`, which spawns its
-- own kind 0x28 for battle mode 9): the navi MegaMan unites with comes in on a spiral around him for 22 ticks
-- (tinted), blinks for 30 and fades out over 8. Its look is its soul's own, the soul's form's `image` (a
-- `SoulImage`): its sprite, the soul's navi's (category 8, by the soul), and what it puts on as it comes (0x0800ED0E
-- with 1: 0x0800EDBC's routines by soul, their parts stepping even while paused) and takes off as it goes (0x0800EEBE
-- with 1): GyroMan's propeller, NapalmMan's cannon, Colonel's cape and KnightMan's ball and chain (the body overlay
-- rows 0, 4, 3 and 1: `image.wearing`), and NumberMan's face (NumberSoul's layer, row 0: 0x0800EE6E, 0x0800EFFA).

local SINE: { number } = require("../math").sine

local image = {}

-- A soul's navi's image's look, for the soul's form's `image`.
function image.look(look: SoulImage): SoulImage
    return define.record("soul-image", look)
end

-- The look of a navi that wears a body overlay's part (an identity of its own wears it): put on as the image comes, its
-- sprite stepping even while paused, and taken off as it goes.
function image.wearing(sprite: SpriteAsset, part: BodyPart): SoulImage
    local worn = define.identity { class = "navi", parts = { body = part } }
    return image.look {
        sprite = sprite,
        put_on = function(me: Object)
            me:add_parts(worn, 1)
        end,
        take_off = function(me: Object)
            me:remove_parts(worn)
        end,
    }
end

export type State = {
    -- The soul (Param1, its number in the original), by its form.
    soul: FormDef,
}

-- Its soul's look (the form's `image`).
local function look_of(me: Object): SoulImage
    local soul = (me.state :: State).soul
    local look: SoulImage? = soul.image
    if look == nil then
        error(string.format("%s names no image (its form's `image`: 0x080C03B4 loads the soul's navi's sprite)", soul.id))
    end
    return look
end

local SOUND = asset.sound("cross-merge")
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
        local take_off = look_of(me).take_off
        if take_off ~= nil then
            take_off(me)
        end
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
    local look = look_of(me)
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:update_sprite()
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    battle.play_sound(SOUND)
    if look.put_on ~= nil then
        look.put_on(me)
    end
    me:set_lifecycle("update")
    tick(me)
end

image.kind = define.kind {
    id = "soul-image",
    pool = "actor",
    state = { soul = "form" },
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

-- 0x080C0524: the image of the soul `soul` (its form) for `owner`, on its side and facing.
--
-- (It is spawned at the registers 0x0801201E's read of the side's transform record leaves (0x08012402): Chaos Unison
-- (+1), the turn's arm chip by number (+6: 0xFFFF without one, which is what this has whatever the chip; a replay's
-- comparison knows the number, exe5-compat) and the turns (+3), which its first tick overwrites but NumberMan's face,
-- spawned before it, copies.)
function image.spawn(owner: Object, soul: FormDef): Object?
    local turns, chaos = owner:form_change_soul()
    local o = battle.spawn(image.kind, Vec3.new(if chaos then 1 else 0, 0xFFFF, turns))
    if o then
        (o.state :: State).soul = soul
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
    end
    return o
end

return image
