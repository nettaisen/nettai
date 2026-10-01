--!strict
-- CircusMan (the original's actor object #0xE, `sub_80BA364`), the navi of
-- the CrcusMan chips, brought by the navi chip controller. He appears,
-- leaps three panels ahead and drops; if he lands on an enemy navi he
-- catches him and holds a show on his panel, six hits 12 ticks apart with a
-- sparkle each; else he waits. Then he rises out of sight. See
-- docs/engine/chips.md §3.6.28.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local crcusman = {}

export type State = {
    -- The controller waiting for him.
    controller: Object?,
}

local SPRITE = asset.sprite("circusman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_CATCH = asset.sound("circus-catch")
local SOUND_DRUMROLL = asset.sound("drumroll")

-- A show's sparkle (effect 0x21).
local SPARKLE = define.effect { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 0 }

-- His actor record (the parts it adds: his two overlays, `sub_8010FD8`).
local ACTOR_TYPE: ActorType = "player"
local AI_INDEX = 0xE

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080BA661

-- Panel flags he needs to stay (as one word), and an enemy navi's body by
-- side (`off_80BA520`).
local CAN_STAND = 0x10010
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

-- His actions.
local APPEAR, STAND, CROUCH, LEAP, CATCH, SHOW, BOW, MISS, RISE, NO_FOOTING =
    0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24

-- `sub_80BA388`: on his panel, on the ground (Z's whole part cleared), no
-- shadow, with his parts.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_navi_parts(ACTOR_TYPE, AI_INDEX, 1)
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4; a word store of the timers when `word`), or
-- count down; true once the timer is 0 or below.
local function countdown(me: Object, ticks: number, word: boolean): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- Z's whole part (the game's halfword stores).
local function add_height(me: Object, pixels: number)
    local p = me.pos
    local whole = int.i16(bit32.rshift(p.z, 16) + pixels)
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.lshift(whole, 16), bit32.band(p.z, 0xFFFF)))
end

-- He goes (both overlays with him) and the controller moves on.
local function gone(me: Object)
    me:remove_navi_parts(ACTOR_TYPE, AI_INDEX)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

-- `sub_80BA414`: he appears; 10 ticks; on, or away if his panel can't hold
-- him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 0x13
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if not countdown(me, 0xA, true) then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then STAND else NO_FOOTING)
end

-- `sub_80BA468`: 30 ticks.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true) then
        me:set_action(CROUCH)
    end
end

-- `sub_80BA490`: 19 ticks.
local function crouch(me: Object)
    if me.phase == 0 then
        me.anim = 0xD
    end
    if countdown(me, 0x13, false) then
        me:set_action(LEAP)
    end
end

-- `sub_80BA4BA`: three panels ahead (a byte store, unchecked), 30 pixels
-- up, down 6 a tick for 5 ticks; on an enemy navi, the catch, else the
-- miss.
local function leap(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.panel_x = me.panel_x + 3 * me.facing
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, bit32.bor(0x1E_0000, bit32.band(p.z, 0xFFFF)))
        me.anim = 0xE
        me.timer = 5
        me.timer2 = 0
        return
    end
    add_height(me, -6)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local caught = bit32.btest(field.flags(me.panel_x, me.panel_y), ENEMY_NAVI[me.alliance])
    me:set_action(if caught then CATCH else MISS)
end

-- `sub_80BA528`: the catch (a pixel down and up); 46 ticks, the drumroll
-- at 21.
local function catch(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0xF
        local p = me.pos
        me.pos = Vec3.new(p.x, bit32.bor(bit32.lshift(int.i16(bit32.rshift(p.y, 16) + 1), 16), bit32.band(p.y, 0xFFFF)), p.z)
        add_height(me, 1)
        battle.play_sound(SOUND_CATCH)
        me.timer = 0x2E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x15 then
        battle.play_sound(SOUND_DRUMROLL)
        return
    end
    if left <= 0 then
        me:set_action(SHOW)
    end
end

-- `sub_80BA686`: a hit on his panel (`sub_80C53A6`: it runs while
-- dimmed).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = 0xFF,
        target = collision.hits_navis,
        self_type = collision.piercing_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80BA6B6`: a sparkle around his panel: up to 31 pixels either side,
-- 16 pixels down the field, 36 up give or take 15 (two draws; each
-- draw's bit 0 picks its sign).
local function sparkle(me: Object)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    local r = battle.rng_positive()
    local dx = bit32.band(r, 0x1F) * 0x10000
    if not bit32.btest(r, 1) then
        dx = -dx
    end
    local r2 = battle.rng_positive()
    local dz = bit32.band(r2, 0xF) * 0x10000
    if not bit32.btest(r2, 1) then
        dz = -dz
    end
    battle.effect(Vec3.new(cx + dx, cy + 0x10_0000, 0x14_0000 + dz + 0x10_0000), SPARKLE)
end

-- `sub_80BA56C`: the show: 72 ticks, a hit and a sparkle every 12 from
-- the first.
local function show(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x10
        me.timer = 0x48
        me.timer2 = 0
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 <= 0 then
        hit(me)
        me.timer2 = 0xC
        sparkle(me)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(BOW)
    end
end

-- `sub_80BA5AA`: 7 ticks.
local function bow(me: Object)
    if me.phase == 0 then
        me.anim = 0x11
    end
    if countdown(me, 7, false) then
        me:set_action(RISE)
    end
end

-- `sub_80BA5D2`: nobody to catch; 20 ticks.
local function miss(me: Object)
    if countdown(me, 0x14, false) then
        me:set_action(RISE)
    end
end

-- `sub_80BA5F6`: up 6 pixels a tick for 5 ticks, then gone.
local function rise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x12
        me.timer = 5
        return
    end
    add_height(me, 6)
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        gone(me)
    end
end

-- `sub_80BA62E`: no footing; 19 ticks, then gone.
local function no_footing(me: Object)
    if me.phase == 0 then
        me.anim = 0xD
    end
    if countdown(me, 0x13, false) then
        gone(me)
    end
end

local ACTIONS = {
    [APPEAR] = appear,
    [STAND] = stand,
    [CROUCH] = crouch,
    [LEAP] = leap,
    [CATCH] = catch,
    [SHOW] = show,
    [BOW] = bow,
    [MISS] = miss,
    [RISE] = rise,
    [NO_FOOTING] = no_footing,
}

crcusman.kind = define.kind {
    id = "crcusman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80BA364`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("CircusMan action %#x reads past his table (off_80BA3EC)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80BA660`: CircusMan for `user`, reporting to `controller` (the
-- CrcusMan chips' navi hook, `off_802CD5C[14]`).
function crcusman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn(crcusman.kind, Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    local s = o.state :: State
    s.controller = controller
    return o
end

return crcusman
