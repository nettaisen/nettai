--!strict
-- The projectile (the original's attack object #0, `sub_80C4E58`): the
-- shot the buster, the charged shot, the cannons and many chips fire. It
-- flies forward one panel every two ticks (the first step on the tick
-- after it appears) and ends on the first thing it hits or when it leaves
-- the field. What differs between its uses is its variant, a record
-- (`projectile.variant { ... }`, the original's `off_80C4C78` rows, which
-- objects/projectile/variants.luau writes out): its collision, element,
-- spark, look, status and bug, what its hit does to the panel, whether
-- it bursts or climbs. Its `row` is a row it also moves on its first step
-- (the spread's side shots).
--
-- Callers fire it with lib/projectile.luau. See
-- docs/engine/objects-and-player.md §B8.

local element = require("../element")
local regions = require("../regions")

local projectile = {}

-- (Each game makes its own kind, `projectile.make_kind`: its burst's sound,
-- effect and collision types. A variant is a "projectile-variant" record.)

export type State = {
    variant: ProjectileVariant,
    -- Rows it also moves on its first step (1 down, -1 up; its second
    -- parameter).
    row: number,
}

-- Ticks it waits on a panel after the one it enters on (Timer2).
local WAIT = 1

-- A game's projectile: its kind's key, and a bursting variant's burst
-- (`sub_80C5014`, `sub_80C5050`): its sound, the effect it shows over the
-- panels, and its hit's collision types (the hit reaches as far as the
-- effects but the panel the projectile hit itself).
export type Look = {
    id: string,
    burst_sound: Sound,
    burst_effect: Effect,
    burst_target: CollisionType,
    burst_self_type: CollisionType,
}

local BURST_HIT_MOD = 3

-- A projectile variant.
function projectile.variant(v: ProjectileVariant): ProjectileVariant
    return new.record("projectile-variant", v)
end

-- `sub_80C5090`: a climbing variant sits a pixel further down the field
-- and a pixel higher on each panel it enters than on the last (its Z
-- keeps what it had).
local function climb(me: Object, v: ProjectileVariant)
    if v.climbs then
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y + 0x10000, p.z + 0x10000)
    end
end

-- `object_clearCollisionRegion`, then the end (the next update frees it).
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C5050` and `sub_80C5014`: the burst on (x, y): the effect over
-- the `shown` region's panels and a hit over `hit`'s. (And a camera
-- shake, which only draws.)
local function burst(me: Object, look: Look, x: number, y: number, shown: Region, hit: Region)
    battle.shake_camera(3, 0x28)
    battle.play_sound(look.burst_sound)
    battle.region_effects(x, y, shown, me.alliance, look.burst_effect)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = hit,
        hit_effect = nil,
        target = look.burst_target,
        self_type = look.burst_self_type,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = BURST_HIT_MOD,
    })
end

-- What the hit does (`loc_80C4F70`): a bursting variant bursts around its
-- panel; then the panel cracks, breaks or changes type, as the variant
-- says.
local function hit(me: Object, look: Look, v: ProjectileVariant)
    local x, y = me.panel_x, me.panel_y
    if v.bursts then
        burst(me, look, x, y, regions.square, regions.ring)
    end
    local h = v.hit_panel
    if h == nil then
        return
    end
    if h.effect == "crack" then
        field.crack(x, y)
    elseif h.effect == "break" then
        field.break_panel(x, y)
    elseif field.solid(x, y) then
        local kind = if me.alliance == 0 then h.left_side else h.right_side
        field.set_type(x, y, kind :: PanelType)
    end
end

-- Wait out the panel, then step forward (and a row, the first time). Off
-- the field it ends: a bursting variant bursts over the last two columns
-- it crossed.
local function fly(me: Object, look: Look, s: State)
    if me.phase_init == 0 then
        me.timer = me.timer2
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.panel_x = me.panel_x + me.facing
    me.panel_y = me.panel_y + s.row
    s.row = 0
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) then
        if s.variant.bursts then
            burst(me, look, x - 2 * me.facing, y, regions.wide_long, regions.wide_long)
        end
        finish(me)
        return
    end
    me:set_coordinates_from_panel()
    climb(me, s.variant)
    me:update_collision_panels()
    me.phase_init = 0
end

-- `sub_80C4F02`: the hits against it resolve and spark; it ends when the
-- battle is over or it hit something, else it flies on.
local function tick(me: Object, look: Look)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        finish(me)
    elseif c.hit_flags ~= 0 then
        hit(me, look, s.variant)
        finish(me)
    else
        fly(me, look, s)
    end
    c:present()
end

-- `sub_80C4E7C`: its look, element and collision from its variant, on its
-- panel. (The game overwrites its link to its owner with its record's
-- address, so the owner is forgotten.)
local function init(me: Object, look: Look)
    local v = (me.state :: State).variant
    me.related1 = nil
    local sprite = v.sprite
    if sprite then
        me.sprite:load(sprite)
        -- sprite_noShadow
        me.sprite.shadow = "with_sprite"
        me.visible = true
        me.anim = v.anim or 0
        me.anim_loaded = 0xFF
    end
    me.element = element.byte(v.element, v.secondary)
    me:set_coordinates_from_panel()
    climb(me, v)
    me.timer2 = WAIT
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.collision.is, v.collision.hits, v.hit_mod)
    local c = me.collision
    c:set_hit_effect(v.spark)
    local status, bug = v.status, v.bug or 0
    if status then
        c.status_base = status
    end
    if bug ~= 0 then
        c.bugs = bit32.bor(bug, bit32.lshift(v.bug_arg or 0, 8))
    end
    c:present()
    me:set_lifecycle("update")
    tick(me, look)
end

-- A game's projectile kind.
function projectile.make_kind(look: Look): Kind
    return new.kind {
        id = look.id,
        pool = "attack",
        state = { variant = "record:projectile-variant", row = "i8" },
        -- `sub_80C4E58`.
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
end

-- `sub_80C4FFE`: a projectile (of `kind`, a game's) of `shot.variant` from `owner` on panel (x,
-- y), with the owner's side and flip; nil if the attack pool is full. It
-- runs later this tick, right after its spawner; its X and Y are its
-- spawner's registers until its init, the same tick, places it, and its Z
-- is the shot's.
function projectile.spawn(kind: Kind, owner: Object, x: number, y: number, shot: ProjectileShot): Object?
    local o = battle.spawn(kind, Vec3.new(0, 0, shot.z))
    if o then
        local s = o.state :: State
        s.variant = shot.variant
        s.row = shot.row or 0
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(shot.damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return projectile
