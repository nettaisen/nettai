--!strict
-- HolyDrem's shot, as EXE5 and EXE4 share it (EXE5's attack object #0x83, 0x080D6A60; EXE4's #0x83, 0x080DD9EC, the
-- same code): a bullet from HolyDrem's stand-in along the user's row, 5 pixels a tick after a 10-tick wait, ending on
-- the first thing it hits or past the field's edge; a thrown piercing break of the stand-in's damage, hit modifier 3,
-- no element. Each game makes its own (`shot.make`) with its look and what of its owner the spawn takes.

local shot = {}

export type Look = {
    -- The kind's id.
    id: string,
    sprite: SpriteAsset,
    -- As it is fired.
    sound: Sound,
    -- What it is and hits (rows 0x17 and 5), and its hits' spark (0).
    is: CollisionType,
    hits: CollisionType,
    spark: Spark,
    -- Its spawn copies its owner's side alone, not its facing (EXE4's 0x080DDB14, whose init then flies it by the
    -- side's way, 0x080DDA44's object_getEnemyDirection, which the unflipped shot faces); none: the side and the
    -- facing (EXE5's 0x080D6B88, whose init flies it by the facing).
    side_only: boolean?,
}

export type Shot = {
    kind: Kind,
    -- EXE5's 0x080D6B88, EXE4's 0x080DDB14: a shot at `pos` from `owner`, with its damage word.
    spawn: (owner: Object, pos: Vec3) -> Object?,
}

-- 16.16 pixels a tick; the wait before it moves.
local SPEED, WAIT = 0x5_0000, 10
local HIT_MOD = 3
-- Its X (in pixels from the field's middle) past which it is off the field.
local EDGE = 160

local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- EXE5's 0x080D6B34, EXE4's 0x080DDAC0: 10 ticks still.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_action(4)
    end
end

-- EXE5's 0x080D6B56, EXE4's 0x080DDAE2: on along the row; past the field's edge it ends.
local function fly(me: Object)
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local x = bit32.rshift(int.u32(me.pos.x), 16)
    if int.u16(x + EDGE) >= 2 * EDGE then
        finish(me)
    end
end

-- EXE5's 0x080D6AF4, EXE4's 0x080DDA80: the hits resolve (its spark); a hit ends it; else its action, and its collision
-- again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
        return
    end
    local a = me.action
    if a == 0 then
        wait(me)
    elseif a == 4 then
        fly(me)
    else
        error(string.format("HolyDrem's shot: action %#x reads past its table (EXE5's 0x080D6B2C)", a))
    end
    c:present()
end

function shot.make(look: Look): Shot
    -- EXE5's 0x080D6A84, EXE4's 0x080DDA10: its sprite, facing its way, no element; a thrown piercing break hitting
    -- navis, the look's spark; its sound.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0xFF
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.element = 0
        me:set_panel_from_coordinates()
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, HIT_MOD)
        me.collision:set_hit_effect(look.spark)
        me.collision:present()
        battle.play_sound(look.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        -- EXE5's 0x080D6A60, EXE4's 0x080DD9EC.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
                return
            end
            if me.active then
                me:update_sprite()
            end
        end,
    }

    local function spawn(owner: Object, pos: Vec3): Object?
        local o = battle.spawn(kind, pos)
        if o then
            o:set_damage_word(bit32.bor(owner.damage, bit32.lshift(owner.stamina, 16)))
            o.alliance = owner.alliance
            if not look.side_only then
                o.flip = owner.flip
            end
            o.related1 = owner
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return shot
