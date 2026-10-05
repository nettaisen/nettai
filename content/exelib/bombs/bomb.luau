--!strict
-- A thrown bomb, as EXE5 and EXE6 share it (EXE6's attack object #8,
-- `sub_80C5BB0`; EXE5's #8): each game makes its own (`bomb.make`, a
-- `BombLook`: its kind, sprite, sound and effects). MiniBomb,
-- EnergBom, MegEnBom and BigBomb. It flies three panels ahead in an arc for
-- 40 ticks, then lands: on a solid panel it blasts its variant's region
-- (with an explosion on each panel) and a tick later ends, leaving what its
-- variant leaves; off the field or on a hole it just ends (a splash in the
-- hole). What differs between the chips is its variant, a record
-- (`bomb.variant { ... }`, `byte_80C5BA0` with the blast regions of
-- `dword_80C5D7C`).
--
-- Until it lands its panel is (0, 0), off the field, so what it presents
-- hits nothing.

local bomb = {}

export type State = { variant: BombVariant }

-- Flight: 40 ticks at 0x2E666 a tick forward, rising at 0x20666 less
-- 0x2800 a tick (`byte_80C5D58`; the gravity sits in the Y velocity).
local FLIGHT_TICKS = 0x28
local SPEED, RISE, GRAVITY = 0x2E666, 0x20666, -0x2800

-- A bomb variant.
function bomb.variant(v: BombVariant): BombVariant
    return define.record("bomb-variant", v)
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C5C9C`: fly; on the last tick, land.
local function fly(me: Object, look: BombLook, v: BombVariant)
    if me.phase_init == 0 then
        me.vel = Vec3.new(SPEED * me.facing, GRAVITY, RISE)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, vel = me.pos, me.vel
    local vz = int.i32(vel.z + vel.y)
    me.pos = Vec3.new(p.x + vel.x, p.y, p.z + vz)
    me.vel = Vec3.new(vel.x, vel.y, vz)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.visible = false
    me:set_panel_from_coordinates()
    local x, y = me.panel_x, me.panel_y
    me.collision.panel_x, me.collision.panel_y = x, y
    if not field.valid(x, y) then
        finish(me)
        return
    end
    if not field.solid(x, y) then
        battle.effect(me.pos, look.splash)
        finish(me)
        return
    end
    me.collision:set_region(v.blast)
    if v.blast then
        battle.region_effects(x, y, v.blast, me.alliance, look.explosion, 0)
    end
    battle.play_sound(look.hit_sound)
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80C5D84`: the tick after landing: leave what the variant leaves,
-- and end.
local function landed(me: Object, v: BombVariant)
    me.collision:set_region(nil)
    if v.after then
        v.after(me)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80C5C40`: resolve last tick's hits; end with the battle or on a
-- hit; stand still while dimmed.
local function tick(me: Object, look: BombLook)
    local v = (me.state :: State).variant
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, look.puff)
        finish(me)
        return
    end
    if not battle.dimmed() then
        if me.collision.hit_flags ~= 0 then
            finish(me)
            return
        end
        local a = me.action
        if a == 0 then
            fly(me, look, v)
        elseif a == 4 then
            landed(me, v)
        else
            error(string.format("bomb action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80C5BD4`.
local function init(me: Object, look: BombLook)
    local v = (me.state :: State).variant
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = v.anim
    me.anim_loaded = 0xFF
    sprite.palette = v.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.collision.is, v.collision.hits, v.hit_mod)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me, look)
end

-- A game's bombs.
function bomb.make(look: BombLook): Bombs
    local kind = define.kind {
        id = look.id,
        pool = "attack",
        state = { variant = "record:bomb-variant" },
        -- `sub_80C5BB0`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }
    return {
        kind = kind,
        thrower = function(variant: BombVariant): Thrower
            return bomb.thrower(kind, variant)
        end,
    }
end

-- `sub_80C5DBC`: a thrower for `throw.action` that throws a bomb (of
-- `kind`, a game's) of `variant`, null-element.
function bomb.thrower(kind: Kind, variant: BombVariant): Thrower
    return {
        variant = variant,
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(kind, pos)
            if o then
                (o.state :: State).variant = variant
                o.element = 0
                o.alliance, o.flip = user.alliance, user.flip
                o:set_damage_word(damage)
                o.related1 = user
            end
            return o
        end,
    }
end

return bomb
