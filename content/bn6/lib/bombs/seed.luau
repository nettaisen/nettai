--!strict
-- A thrown seed (the original's attack object #0x4F, `sub_80CE24C`):
-- PoisSeed, IceSeed and GrasSeed. It flies three panels ahead in the bomb's
-- arc and lands: on a solid panel it hits it for a tick, then turns the
-- panels around it into its variant's kind of panel, only its own if the
-- hit landed on something; off the field or on a hole it just ends (a
-- splash in the hole). What differs between the seeds is its variant
-- (`dword_80CE2B0`, `byte_80CE41E`, `byte_80CE421`, `byte_80CE418` and the
-- chip's palette).

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local effects = require("../effects")
local panels = require("../panels")
local regions = require("../regions")
local sparks = require("../sparks")

local seed = {}

export type State = { variant: SeedVariant }

-- `sub_8011504`: sprite `bomb`, animation 9, with a ground shadow.
local SPRITE = asset.sprite("bn6:bomb")
local ANIM = 9
local HIT_MOD = 1
-- Flight: as the bomb's (`byte_80CE3AC`).
local FLIGHT_TICKS = 0x28
local SPEED, RISE, GRAVITY = 0x2E666, 0x20666, -0x2800

-- What grows on each panel: a sprout (poison, ice) or a green one (grass)
-- (effects 0x3B, 0x3C).
seed.sprout = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 0 }
seed.green_sprout = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 8 }

-- What the navi holds while throwing a seed: the bomb's sprite in the
-- seed's colours (the original's attachment rows 0x24, 0x25, 0x26).
function seed.held(palette: number): AttachmentLook
    return attachment.look { sprite = SPRITE, palette = palette }
end

-- A seed variant.
function seed.variant(v: SeedVariant): SeedVariant
    return define.record("seed-variant", v)
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80CE308`: fly; on the last tick, land.
local function fly(me: Object)
    if me.phase_init == 0 then
        me.vel = Vec3.new(SPEED * me.facing, GRAVITY, RISE)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + v.y)
    me.pos = Vec3.new(p.x + v.x, p.y, p.z + vz)
    me.vel = Vec3.new(v.x, v.y, vz)
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
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        me.visible = false
        return
    end
    if not field.solid(x, y) then
        battle.effect(me.pos, effects.splash)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        me.visible = false
        return
    end
    me.collision:set_region(regions.single)
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CE3C4`: the tick after landing: grow the panels, and end.
local function burst(me: Object)
    local v = (me.state :: State).variant
    me.collision:set_region(nil)
    local r = if me.collision.hit_flags ~= 0 then regions.single else regions.square
    local x, y = me.panel_x, me.panel_y
    -- (The game passes the side and flip halfword as the side, unmasked,
    -- so a flipped seed's dx lands far off the field.)
    local dir = panels.side_direction(me.alliance + me.flip * 0x100)
    regions.effects_on_panels(x, y, v.effect, r, dir, 0)
    regions.set_types(x, y, v.panel, r)
    battle.play_sound(v.sound)
    me:set_lifecycle("destroy")
end

-- `sub_80CE2B4`: resolve last tick's hits; end with the battle; stand
-- still while dimmed.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, effects.puff)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            burst(me)
        else
            error(string.format("seed action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80CE270`.
local function init(me: Object)
    local v = (me.state :: State).variant
    me.element = v.element
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = ANIM
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    sprite.palette = v.palette
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

seed.kind = define.kind {
    id = "bn6:seed",
    pool = "attack",
    state = { variant = "record:seed-variant" },
    -- `sub_80CE24C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80CE44E`: a thrower for `throw.action` that throws a seed of
-- `variant`.
function seed.thrower(variant: SeedVariant): Thrower
    return {
        variant = variant,
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(seed.kind, pos)
            if o then
                (o.state :: State).variant = variant
                o.alliance, o.flip = user.alliance, user.flip
                o:set_damage_word(damage)
                o.related1 = user
            end
            return o
        end,
    }
end

return seed
