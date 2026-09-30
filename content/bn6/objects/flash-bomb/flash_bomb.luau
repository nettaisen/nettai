--!strict
-- A FlashBomb (attack object #0xA4, `sub_80D9A50`): FlshBom1-3 (action
-- 0x12). Thrown three panels ahead in 40 ticks, it sets down on a solid,
-- free panel as an object with HP and counts down a minute; two ticks
-- before the end the screen flashes and a hit with a status (blinding)
-- covers the other side's area, and levels with a second status hit the
-- area again as it goes. Breaking it first (its HP) or a hit that
-- crushes it ends it quietly. Its level is its first parameter, the side
-- it flashes (against its own) the second, and the panel it aims at the
-- third and fourth.

local hp = require("../../lib/hp")
local trajectory = require("../../lib/trajectory")

local flash_bomb = {}

-- `sub_8011504`: sprite 0x10-0x52 with a ground shadow, animation 0;
-- collision self type 0x0A, target 5, hit modifier 3.
local SPRITE = "10-52"
local SELF_TYPE, TARGET, HIT_MOD = 0x0A, 0x05, 3
-- Set down: self type 0x0C, target 0x0D, hit modifier 3.
local SET_SELF_TYPE, SET_TARGET, SET_HIT_MOD = 0x0C, 0x0D, 3
local FLIGHT_TICKS, GRAVITY = 0x28, -0x3000
local EFFECT_SPLASH, EFFECT_GONE = 1, 0x14
-- What in its hits ends it at once: bodies and objects (0x0F800000).
local CRUSHED = 0x0F800000
-- A body of its own side on the panel keeps it from setting down.
local OWN_BODY = { [0] = 0x08000000, [1] = 0x04000000 }
-- The flash: two ticks before the end, a palette flash (variant 1, 6
-- ticks) and sound 0x1BD; the hit covers the other side's area (whole-
-- field region 0x82 for side 0, 0x81 for side 1, `byte_80D9D24`) as
-- self type 0x0B; levels 3 and up hit with damage 0x14 and counter 0.
local FLASH_AT, SOUND_FLASH = 2, 0x1BD
local FLASH_REGIONS = { [0] = 0x82, [1] = 0x81 }
local FLASH_SELF_TYPE, FLASH_TARGET = 0x0B, 0x05
local STRONG_DAMAGE = 0x140000

-- By level (`byte_80D9A20`): ticks set down, HP, the flash's status, the
-- status of a second hit as it goes (0 none), and palette.
type Level = { ticks: number, hp: number, status: number, final_status: number, palette: number }
local LEVELS: { [number]: Level } = {
    [0] = { ticks = 0x3C, hp = 0x0A, status = 0x10, final_status = 0x00, palette = 0x0 },
    [1] = { ticks = 0x3C, hp = 0x0F, status = 0x12, final_status = 0x00, palette = 0x3 },
    [2] = { ticks = 0x3C, hp = 0x14, status = 0x12, final_status = 0x30, palette = 0x6 },
    [3] = { ticks = 0x3C, hp = 0x04, status = 0x10, final_status = 0x00, palette = 0x0 },
    [4] = { ticks = 0x3C, hp = 0x08, status = 0x30, final_status = 0x00, palette = 0x3 },
    [5] = { ticks = 0x3C, hp = 0x0C, status = 0x20, final_status = 0x00, palette = 0x6 },
    [6] = { ticks = 0x3C, hp = 0x10, status = 0x11, final_status = 0x00, palette = 0x9 },
    [7] = { ticks = 0x3C, hp = 0x0C, status = 0x11, final_status = 0x00, palette = 0xC },
    [8] = { ticks = 0x3C, hp = 0x14, status = 0x12, final_status = 0x00, palette = 0xF },
}

local function level(me: Object): Level
    local l = LEVELS[me:param(1)]
    if not l then
        error(string.format("FlashBomb level %d reads past byte_80D9A20", me:param(1)))
    end
    return l
end

-- `sub_80D9CC2`: a FlashBomb thrown by `user` from `pos` at the panel
-- three ahead, with the chip's parameters and damage word. (The target
-- is or-ed into the third and fourth parameters, as a word.)
function flash_bomb.throw(user: Object, pos: Vec3, params: { number }, damage: number): Object?
    local target = bit32.bor(int.shl(user.panel_x + 3 * user.facing, 16), int.shl(user.panel_y, 24))
    local word = bit32.bor(params[1], int.shl(params[2], 8), int.shl(params[3], 16), int.shl(params[4], 24), target)
    local bytes = { bit32.band(word, 0xFF), bit32.extract(word, 8, 8), bit32.extract(word, 16, 8), bit32.extract(word, 24, 8) }
    local o = battle.spawn_kind("flash-bomb", pos, bytes)
    if o then
        o.alliance, o.flip = user.alliance, user.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.element = 0
        o.related1 = user
    end
    return o
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- Done after setting down or trying to: its reservation goes too.
local function leave(me: Object)
    me.collision.region = 0
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    me:set_lifecycle("destroy")
    me.visible = false
end

-- `sub_80D9CE4`: the flash's hit, with status `status` and the damage
-- word `damage` (a stun status 0x10..0x15 comes without the hit
-- modifier).
local function flash(me: Object, status: number, damage: number)
    local hit_mod = if status >= 0x10 and status <= 0x15 then 0 else 1
    if me:param(1) >= 3 then
        damage = STRONG_DAMAGE
    end
    local side = bit32.bxor(me.alliance, me:param(2))
    local r = FLASH_REGIONS[side]
    if r == nil then
        error(string.format("FlashBomb side %d reads past byte_80D9D24", side))
    end
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = r,
        hit_effect = 0xFF,
        target = FLASH_TARGET,
        self_type = FLASH_SELF_TYPE,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = hit_mod,
        status = bit32.band(status, 0xFF),
    })
end

-- `sub_80D9B2C`: fly to the target panel; on the last tick, set down if
-- the panel is solid and has no body of its own side.
local function fly(me: Object)
    if me.phase_init == 0 then
        local tx, ty = field.center(me:param(3), me:param(4))
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
        battle.effect(me.pos, EFFECT_SPLASH)
        leave(me)
        return
    end
    me.collision.region = 1
    me:set_coordinates_from_panel()
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9C1C`: set down on the panel with its HP and count.
local function set_down(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:reset_collision_types(SET_SELF_TYPE, SET_TARGET, SET_HIT_MOD)
    me.anim = 1
    local l = level(me)
    me.timer = l.ticks + 1
    me.hp, me.max_hp = l.hp, l.hp
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    me.action = 8
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D9C54`: count down; flash two ticks before the end; go, with a
-- second hit for levels that have one.
local function count_down(me: Object)
    local l = level(me)
    if me.timer == FLASH_AT then
        battle.palette_flash(1, 6)
        battle.play_sound(SOUND_FLASH)
        flash(me, l.status, int.u32(me.damage + me.stamina * 0x10000))
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    if l.final_status ~= 0 then
        flash(me, l.final_status, 0)
    end
    leave(me)
end

-- `sub_80D9ABA`: resolve last tick's hits; crushed, broken or the battle
-- over, it ends; while dimmed it stands.
local function tick(me: Object)
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
        battle.effect(me.pos, EFFECT_GONE)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            set_down(me)
        elseif a == 8 then
            count_down(me)
        else
            error(string.format("FlashBomb action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80D9A74`.
local function init(me: Object)
    local l = level(me)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    -- `sub_801A146`: the hit spark is the element's.
    me.collision.hit_effect = bit32.band(me.element, 0xF)
    me.collision.region = 0
    me.collision:present()
    sprite.palette = l.palette
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D9A50`.
function flash_bomb.update(me: Object)
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
end

return flash_bomb
