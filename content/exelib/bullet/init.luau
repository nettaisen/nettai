--!strict
-- The bullet, as EXE5 and EXE6 share it (EXE6's attack object #0x12,
-- `sub_80C6946`; EXE5's #0x12, 0x080C6B98): the
-- Vulcans' and Spreaders' shot, SpoutCross's charged shot, ColArmy's and
-- ColForce's soldiers'. An unseen probe that steps a panel a tick from where it
-- appears; on the first thing it would hit it bursts there: its variant's
-- region is hit once (the tick after it finds the target), with an effect
-- shown over the region or at the bullet. What differs between its uses
-- is its variant (`bullet.variant { ... }`, the original's `byte_80C68D4`
-- rows). See docs/engine/shot-chips.md §4.1. Each game makes its kind with
-- `bullet.make_kind(id, look)` (the collision types it probes and hits
-- with) and fires it with `bullet.fire`.

local bullet = {}

export type State = {
    variant: BulletVariant,
}

-- A bullet variant.
function bullet.variant(v: BulletVariant): BulletVariant
    return new.record("bullet-variant", v)
end

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80C6A08`: a panel forward; off the field it ends.
local function step(me: Object)
    me.panel_x = me.panel_x + me.facing
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80C6A50`: the burst: its hit over the variant's region on the panel
-- it found (`sub_80C6AB8`: an attack of its own element now, which hits
-- with the variant's modifier and status), the effect, the sound.
local function burst(me: Object, v: BulletVariant, look: BulletLook)
    me:reset_collision_types(look.attack, look.target, v.hit_mod)
    me.collision.element = bit32.band(me.element, 0xF)
    me.collision.secondary_element = bit32.band(me.element, 0xF0)
    local status = v.status
    if status then
        me.collision.status_base = status
    end
    local c = me.collision
    c:set_region(v.region)
    if v.single then
        battle.effect(me.pos, v.effect, bit32.bxor(me.alliance, me.flip))
    else
        battle.region_effects(c.panel_x, c.panel_y, v.region, me.alliance, v.effect, me.pos.z)
    end
    if v.sound then
        battle.play_sound(v.sound)
    end
    me.phase, me.phase_init = 4, 0
end

-- `sub_80C69AC`: the hits against it resolve and spark; the first time it
-- finds something (its DEAD status a latch) it bursts there.
local function tick(me: Object, look: BulletLook)
    local v = (me.state :: State).variant
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 and not me:status("dead") then
        me:set_status("dead", true)
        me.action = 4
        me.phase, me.phase_init = 0, 0
    end
    if me.action == 0 then
        step(me)
    elseif me.phase == 0 then
        burst(me, v, look)
    else
        -- `sub_80C6AAC`: its hit landed; done.
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
    c:present()
end

-- `sub_80C6964`: on its panel, unseen, a probe (§1 of shot-chips.md: it
-- finds what an attack would hit without hurting it, element null).
local function init(me: Object, look: BulletLook)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.probe, look.target, 0)
    me.collision.element = 0
    me.collision:present()
    me:set_lifecycle("update")
end

-- A bullet kind (`id`) of `look`.
function bullet.make_kind(id: string, look: BulletLook): Kind
    return new.kind {
        id = id,
        pool = "attack",
        state = { variant = "record:bullet-variant" },
        -- `sub_80C6946`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80C6ADA`: a bullet (of `kind`, a game's) of `variant` from `owner`
-- on panel (x, y) at height `z` (16.16), with `element` and the damage
-- word; nil if the attack pool is full. It takes the owner's side and flip,
-- runs later this tick, right after its spawner, and keeps its Z.
function bullet.fire(kind: Kind, owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number): Object?
    local o = battle.spawn(kind, Vec3.new(0, 0, z))
    if o then
        (o.state :: State).variant = variant
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return bullet
