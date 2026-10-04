--!strict
-- A FlashBomb (the original's attack object #0xA4, `sub_80D9A50`): FlshBom1-3.
-- Thrown three panels ahead in 40 ticks, it sets down on a solid, free
-- panel as an object with HP and counts down a minute; two ticks before the
-- end the screen flashes and a hit with a status (blinding) covers the
-- other side's area, and variants with a second status hit the area again
-- as it goes. Breaking it first (its HP) or a hit that crushes it ends it
-- quietly. What differs between the chips is its variant
-- (`byte_80D9A20`'s row, the chip's level).

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local hp = require("../../lib/hp")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local flash_bomb = {}

export type State = {
    variant: FlashBombVariant,
    -- The panel it aims at.
    target_x: number,
    target_y: number,
}

-- `sub_8011504`: sprite `flash-bomb` with a ground shadow, animation 0.
local SPRITE = asset.sprite("flash-bomb")
local HIT_MOD, SET_HIT_MOD = 3, 3
local FLIGHT_TICKS, GRAVITY = 0x28, -0x3000
-- What in its hits ends it at once: bodies and objects (0x0F800000).
local CRUSHED = 0x0F800000
-- A body of its own side on the panel keeps it from setting down.
local OWN_BODY = { [0] = 0x08000000, [1] = 0x04000000 }
-- The flash: two ticks before the end, a palette flash (variant 1, 6
-- ticks) and a sound; the hit covers the other side's area; strong
-- variants hit with damage 0x14 and counter 0.
local FLASH_AT = 2
local SOUND_FLASH = asset.sound("flash")
local STRONG_DAMAGE = 0x140000

-- What the navi holds while throwing one (the original's attachment row
-- 0x2E); each level adds 3 to its palette.
flash_bomb.held = attachment.look { sprite = SPRITE, palette = 0 }

-- A FlashBomb variant.
function flash_bomb.variant(v: FlashBombVariant): FlashBombVariant
    return define.record("flash-bomb-variant", v)
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- Done after setting down or trying to: its reservation goes too.
local function leave(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
    me.visible = false
end

-- `sub_80D9CE4`: the flash's hit, with status `status` and the damage
-- word `damage` (a paralyzing status, the original's 0x10..0x15, comes
-- without the hit modifier).
local function flash(me: Object, v: FlashBombVariant, status: Status, damage: number)
    local hit_mod = if status.timer == "paralyze" then 0 else 1
    if v.strong then
        damage = STRONG_DAMAGE
    end
    -- The side it flashes against (`byte_80D9D24`): the other side's area.
    local side = bit32.bxor(me.alliance, if v.own_side then 1 else 0)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.area[bit32.bxor(side, 1)],
        -- None.
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.piercing,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = hit_mod,
        status = status,
    })
end

-- `sub_80D9B2C`: fly to the target panel; on the last tick, set down if
-- the panel is solid and has no body of its own side.
local function fly(me: Object, s: State)
    if me.phase_init == 0 then
        local tx, ty = field.center(s.target_x, s.target_y)
        me.vel = trajectory.in_ticks(me.pos, tx, ty, 0, FLIGHT_TICKS, GRAVITY)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z + v.z)
    me.vel = Vec3.new(v.x, v.y, v.z + GRAVITY)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me:set_panel_from_coordinates()
    local x, y = me.panel_x, me.panel_y
    me.collision.panel_x, me.collision.panel_y = x, y
    if not field.valid(x, y) then
        leave(me)
        return
    end
    if not field.solid(x, y) or bit32.btest(field.flags(x, y), OWN_BODY[me.alliance]) then
        battle.effect(me.pos, effects.splash)
        leave(me)
        return
    end
    me.collision:set_region(regions.single)
    me:set_coordinates_from_panel()
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9C1C`: set down on the panel with its HP and count.
local function set_down(me: Object, v: FlashBombVariant)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:reset_collision_types(collision.own_object, collision.hit_by_other_side_or_blockers, SET_HIT_MOD)
    me.anim = 1
    me.timer = v.ticks + 1
    me.hp, me.max_hp = v.hp, v.hp
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    me.action = 8
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9C54`: count down; flash two ticks before the end; go, with a
-- second hit for variants that have one.
local function count_down(me: Object, v: FlashBombVariant)
    if me.timer == FLASH_AT then
        battle.palette_flash(1, 6)
        battle.play_sound(SOUND_FLASH)
        flash(me, v, v.status, int.u32(me.damage + me.stamina * 0x10000))
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local final = v.final_status
    if final then
        flash(me, v, final, 0)
    end
    leave(me)
end

-- `sub_80D9ABA`: resolve last tick's hits; crushed, broken or the battle
-- over, it ends; while dimmed it stands.
local function tick(me: Object)
    local s = me.state :: State
    me.collision:remove()
    me:hit_spark()
    if bit32.btest(me.collision.hit_flags, CRUSHED) then
        finish(me)
        return
    end
    if me.action == 8 and hp.apply_damage(me, 0) == -1 then
        finish(me)
        return
    end
    if battle.over() then
        battle.effect(me.pos, effects.puff)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me, s)
        elseif a == 4 then
            set_down(me, s.variant)
        elseif a == 8 then
            count_down(me, s.variant)
        else
            error(string.format("FlashBomb action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80D9A74`.
local function init(me: Object)
    local v = (me.state :: State).variant
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:set_region(nil)
    me.collision:present()
    sprite.palette = v.palette
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

flash_bomb.kind = define.kind {
    id = "flshbom/bomb",
    pool = "attack",
    state = { variant = "record:flash-bomb-variant", target_x = "u8", target_y = "u8" },
    -- `sub_80D9A50`.
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

-- `sub_80D9CC2`: a thrower for `throw.action` that throws a FlashBomb of
-- `variant` at the panel three ahead of the user, null-element.
function flash_bomb.thrower(variant: FlashBombVariant): Thrower
    return {
        variant = variant,
        throw = function(user: Object, pos: Vec3, damage: number): Object?
            local o = battle.spawn(flash_bomb.kind, pos)
            if o then
                local s = o.state :: State
                s.variant = variant
                s.target_x = user.panel_x + 3 * user.facing
                s.target_y = user.panel_y
                o.alliance, o.flip = user.alliance, user.flip
                o:set_damage_word(damage)
                o.element = 0
                o.related1 = user
            end
            return o
        end,
    }
end

return flash_bomb
