--!strict
-- CircusMan (the original's actor object #0xE, `sub_80BA364`), the navi of
-- the CrcusMan chips, brought by the navi chip controller. He appears,
-- leaps three panels ahead and drops; if he lands on an enemy navi he
-- catches him and holds a show on his panel, six hits 12 ticks apart with a
-- sparkle each; else he waits. Then he rises out of sight. See
-- docs/engine/chips.md §3.6.28.

local navi_chips = require("../../lib/navi-chips/navi")
local collision = require("../../rules/collision")
local regions = require("@common/regions")

local crcusman = {}

export type State = {
    -- The controller waiting for him.
    controller: Object?,
}

local SPRITE = asset.sprite("bn6:circusman")
local SOUND_APPEAR = asset.sound("bn6:appear")
local SOUND_CATCH = asset.sound("bn6:circus-catch")
local SOUND_DRUMROLL = asset.sound("bn6:drumroll")

-- A show's sparkle (effect 0x21).
local SPARKLE = define.effect { sprite = asset.sprite("bn6:pink-flash"), anim = 0, palette = 0 }

-- What he is taken for: his actor record, and what he wears (his two overlays, `sub_8010FD8`).
local IDENTITY = define.identity {
    id = "bn6:circusman",
    class = "navi",
    version = 0,
    ai_index = 0xE,
    ice = "large",
    parts = {
        body = {
            sprite = asset.sprite("bn6:circusman"),
            anims = 100,
            behind = { { 0x00, 0x13 }, { 0x28, 0x4F }, { 0x63, 0x63 } },
            anim_offset = 0x14,
        },
        second = {
            sprite = asset.sprite("bn6:circusman"),
            anims = 80,
            behind = { { 0x14, 0x3B }, { 0x4F, 0x4F } },
            anim_offset = 0x28,
        },
    },
}

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080BA661

-- An enemy navi's body by side (`off_80BA520`).
local ENEMY_NAVI = { [0] = 0x0400_0000, [1] = 0x0800_0000 }

-- His actions.
local APPEAR, STAND, CROUCH, LEAP, CATCH, SHOW, BOW, MISS, RISE, NO_FOOTING =
    0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24

-- `sub_80BA388`: on his panel, on the ground (Z's whole part cleared), no
-- shadow, with his parts.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "with_sprite")
    me:add_parts(IDENTITY, 1)
    me:set_lifecycle("update")
end

-- Z's whole part (the game's halfword stores).
local function add_height(me: Object, pixels: number)
    local p = me.pos
    local whole = int.i16(bit32.rshift(p.z, 16) + pixels)
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.lshift(whole, 16), bit32.band(p.z, 0xFFFF)))
end

-- He goes (both overlays with him) and the controller moves on.
local function gone(me: Object)
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

-- `sub_80BA414`: he appears; 10 ticks; on, or away if his panel can't hold
-- him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 0x13
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if not navi_chips.wait(me, 0xA, true) then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then STAND else NO_FOOTING)
end

-- `sub_80BA468`: 30 ticks.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if navi_chips.wait(me, 0x1E, true) then
        me:set_action(CROUCH)
    end
end

-- `sub_80BA490`: 19 ticks.
local function crouch(me: Object)
    if me.phase == 0 then
        me.anim = 0xD
    end
    if navi_chips.wait(me, 0x13, false) then
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
        hit_effect = nil,
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
    if navi_chips.wait(me, 7, false) then
        me:set_action(RISE)
    end
end

-- `sub_80BA5D2`: nobody to catch; 20 ticks.
local function miss(me: Object)
    if navi_chips.wait(me, 0x14, false) then
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
    if navi_chips.wait(me, 0x13, false) then
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
    id = "bn6:crcusman/navi",
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
    local o = navi_chips.spawn(crcusman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return crcusman
