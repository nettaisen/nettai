--!strict
-- A soul's image, as EXE5 and EXE4 share it (EXE5's actor object 0x2A, 0x080C0394, spawned by 0x080C0524; EXE4's effect
-- object 0x13, 0x080E361C, spawned by 0x080E37D8): the navi MegaMan unites with comes in on a spiral around him for
-- 22 ticks (tinted), blinks for 30 and fades out over 8. Its look is its soul's own, the soul's form's `image` (a
-- `SoulImage`): its sprite, the soul's navi's (category 8, by the soul), and what it puts on as it comes and takes off
-- as it goes.
--
-- Each game makes its own kind (`image.make`): EXE5's an actor, placed two pixels up and on from its owner and running
-- its first tick as it starts; EXE4's an effect, on its owner, its first tick the one after its start (0x080E363C sets
-- its state and returns).

local image = {}

-- A soul's navi's image's look, for the soul's form's `image`.
function image.look(look: SoulImage): SoulImage
    return new.record("soul-image", look)
end

-- The look of a navi that wears a body overlay's part (an identity of its own wears it): put on as the image comes, its
-- sprite stepping even while paused, and taken off as it goes.
function image.wearing(sprite: SpriteAsset, part: BodyPart): SoulImage
    local worn = new.identity { class = "navi", parts = { body = part } }
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

export type Spec = {
    -- The kind's id, and the pool it is of.
    id: string,
    pool: "actor" | "effect",
    -- The sound as it comes (EXE5's 0x080C03E8, EXE4's 0x080E369A).
    sound: Sound,
    -- The game's sine table (its rules/math's: 384 values, the cosine 64 on).
    sine: { number },
    -- Where it sits from its owner besides the spiral (across, up the field, in height), in subpixels: EXE5's two
    -- pixels up and on (0x080C04F2), EXE4's none (0x080E37B2).
    offset: Vec3,
    -- Its init runs its first tick too (EXE5's 0x080C03B4 goes on into 0x080C040E).
    ticks_at_init: boolean,
    -- Its spawn takes its owner's side alone, not its facing (EXE4's 0x080E37EA).
    side_only: boolean,
    -- It runs while the battle is paused (EXE5's 0x080C0524 sets it; EXE4's runs while dimmed, as its pool's objects
    -- start).
    runs_while_paused: boolean,
    -- Its sprite's priority, where the game's isn't its sprite's own: EXE4's 0 as it comes, in front of the HUD's
    -- sprites (the chip icons over MegaMan, priority 1: its parts' OAM entries as the image blinks, a RAM dump of the
    -- chip lab's souls/roll/unison at frame 502), and 2 as it fades out (0x080E3776), behind them. None: EXE5's, its
    -- sprite's own throughout.
    priority: number?,
    fade_priority: number?,
}

export type Image = {
    kind: Kind,
    -- The image of the soul `soul` (its form) for `owner`, on its side, spawned at `at` (the caller's registers).
    spawn: (owner: Object, soul: FormDef, at: Vec3) -> Object?,
}

-- The tint while it spirals in.
local TINT = 0x2108

-- Its soul's look (the form's `image`).
local function look_of(me: Object): SoulImage
    local soul = (me.state :: State).soul
    local look: SoulImage? = soul.image
    if look == nil then
        error(string.format("%s names no image (its form's `image`: a soul's image loads the soul's navi's sprite)", soul.id :: string))
    end
    return look
end

function image.make(spec: Spec): Image
    local function sine(i: number): number
        return spec.sine[i + 1]
    end

    -- EXE5's 0x080C04F2, EXE4's 0x080E37B2: around its owner at `radius` pixels and angle `angle` (256 steps a turn)
    -- across and in height, and the spec's offset.
    local function place(me: Object, radius: number, angle: number)
        local owner = me.related1 :: Object
        local dx = int.shl(sine(angle + 64) * radius, 8)
        local dz = int.shl(sine(angle) * radius, 8)
        local p, o = owner.pos, spec.offset
        me.pos = Vec3.new(p.x + dx + o.x, p.y + o.y, p.z + dz + o.z)
    end

    -- EXE5's 0x080C042C, EXE4's 0x080E36DC: 22 ticks spiraling in.
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

    -- EXE5's 0x080C046C, EXE4's 0x080E3724: 30 ticks blinking, two ticks of four.
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

    -- EXE5's 0x080C04AE, EXE4's 0x080E3766: back on MegaMan, 8 ticks fading out; then it goes.
    local function fade(me: Object)
        if me.phase_init == 0 then
            place(me, 0, 0)
            if spec.fade_priority ~= nil then
                me.sprite.priority = spec.fade_priority
            end
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

    -- EXE5's 0x080C040E, EXE4's 0x080E36BC
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

    -- EXE5's 0x080C03B4, EXE4's 0x080E363C
    local function init(me: Object)
        local look = look_of(me)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "hidden"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        me:update_sprite()
        sprite.palette = 0
        if spec.priority ~= nil then
            sprite.priority = spec.priority
        end
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.visible = true
        battle.play_sound(spec.sound)
        if look.put_on ~= nil then
            look.put_on(me)
        end
        me:set_lifecycle("update")
        if spec.ticks_at_init then
            tick(me)
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = spec.pool,
        state = { soul = "form" },
        -- EXE5's 0x080C0394, EXE4's 0x080E361C
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

    -- EXE5's 0x080C0524, EXE4's 0x080E37D8: the image of the soul `soul` for `owner`, on its side and facing.
    local function spawn(owner: Object, soul: FormDef, at: Vec3): Object?
        local o = battle.spawn(kind, at)
        if o then
            (o.state :: State).soul = soul
            o.related1 = owner
            o.alliance = owner.alliance
            if not spec.side_only then
                o.flip = owner.flip
            end
            if spec.runs_while_paused then
                o.run_while_paused = true
            end
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return image
