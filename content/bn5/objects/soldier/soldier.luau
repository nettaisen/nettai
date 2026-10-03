--!strict
-- ColonelSoul's soldiers (BN5's attack object #0x30, 0x080CA834; BN6's ColArmy soldier's number, BN5's own code):
-- what an obstacle turns into where its side's enemy's ColonelSoul can use it (the obstacle framework's step,
-- 0x080CAB02: the rule `effects.obstacle_soldiers` and the role `kinds.obstacle_soldier`, docs/design/bn5-map.md
-- §15.11). The engine spawns it on the obstacle's panel for the armed side, `gun` its Param1:
--
-- - the sword's soldier (`gun` 0: an enemy between the obstacle and the side's back; it faces back) draws for 15 ticks
--   and slashes the two panels ahead 6 ticks into the next 24, with the side's first damage word;
-- - the gun's soldier (`gun` 1: an enemy ahead on the row) takes aim for 15 ticks and fires three shots (the Vulcans'
--   bullets) 10 ticks apart at a random one of four heights, with the side's second word.
--
-- Each reads its word as it strikes (`obstacle.soldiers`), then blinks out over 30 ticks, or at once when a hit takes
-- its 1 HP. Its element byte is what the obstacle's search left (0x1D for the sword's, 0 for the gun's).

local bullet = require("../bullet/bullet")
local collision = require("../../rules/collision")
local common_bullet = require("@common/bullet/bullet")
local marker = require("../../lib/navi-chips/marker")
local regions = require("@common/regions")

local soldier = {}

export type State = {
    -- Its Param1: 0 the sword's soldier, 1 the gun's.
    gun: number,
    -- +0x60: a hit took its HP (and it blinks out).
    hit: boolean,
}

-- 0x080CA858: sprite 10-20 with a shadow (`sub_8011504`'s 0x01001020 and 0x01031020: animation 0 or 3).
local SPRITE = asset.sprite("bn5:colonel-soldier")
local HP = 1
-- Its collision (0x13140300): its side's body (row 0x13) that the other side reaches (row 0x14), hit modifier 3.
local HIT_MOD = 3
local DRAW_SOUND, SHOT_SOUND = asset.sound("bn5:sword-swing"), asset.sound("bn5:col-army-2")
-- The slash: over the panel ahead and the one past it (region 2), no spark (0xFF), a slash (row 7) against navis
-- (row 5), hit modifier 3 (r7); its look 16 pixels up, effect 0x17 of BN5's table (0x080DFC94: sprite 0C-14, the
-- swords' slash, animation 1).
local SLASH_TICKS, SLASH_AT = 24, 18
local SLASH = define.effect { sprite = asset.sprite("bn5:sword-slash"), anim = 1, palette = 0 }
local SLASH_Z = 0x10_0000
-- The shots: the Vulcans' bullets (row 0xC), three 10 ticks apart, at one of four heights (0x080CAA84, pixels).
local SHOTS, SHOT_TICKS = 3, 10
local HEIGHTS = { [0] = 0x08, 0x10, 0x18, 0x20 }
-- Before striking, each marks the panel ahead in the battle flag 0x40 mode (0x080E9FD2: ticks, style).
local DRAW_TICKS, AIM_TICKS = 15, 15
local LEAVE_TICKS = 30

-- The actions (0x080CA8F0).
local SWORD, GUN, LEAVE = 0, 4, 8

-- Count the timer down (the halfword's signed result); true when it runs out.
local function count(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- To the next action, from its first phase (the action byte, then the phase halfword).
local function next_action(me: Object, action: number)
    me.action = action
    me.phase, me.phase_init = 0, 0
end

-- 0x080CA918: the sword's soldier draws.
local function draw(me: Object)
    if me.phase_init == 0 then
        me.anim = 1
        me.timer = DRAW_TICKS
        battle.play_sound(DRAW_SOUND)
        marker.spawn(me, me.panel_x + me.facing, me.panel_y, 2, 10, true)
        me.phase_init = 4
    end
    if count(me) then
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x080CA952: the slash.
local function slash(me: Object)
    if me.phase_init == 0 then
        me.anim = 2
        me.timer = SLASH_TICKS
        me.phase_init = 4
    end
    if me.timer == SLASH_AT then
        local _, word = obstacle.soldiers(me.alliance)
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = 0,
            region = regions.long,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = HIT_MOD,
        })
        local px, py = field.center(x, y)
        battle.effect(Vec3.new(px, py, SLASH_Z), SLASH, bit32.bxor(me.alliance, me.flip))
    end
    if count(me) then
        me.collision:set_region(nil)
        next_action(me, LEAVE)
    end
end

-- 0x080CA9DC: the gun's soldier takes aim.
local function aim(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
        me.timer = AIM_TICKS
        marker.spawn(me, me.panel_x + me.facing, me.panel_y, 45, 8, true)
        me.phase_init = 4
    end
    if count(me) then
        me.phase, me.phase_init = 4, 0
    end
end

-- 0x080CAA10: three shots.
local function fire(me: Object)
    if me.phase_init == 0 then
        me.anim = 5
        me.timer2 = SHOTS
        me.timer = 0
        me.phase_init = 4
    end
    if not count(me) then
        return
    end
    me.timer = SHOT_TICKS
    local _, _, word = obstacle.soldiers(me.alliance)
    local z = bit32.lshift(HEIGHTS[bit32.band(battle.rng_positive(), 3)], 16)
    common_bullet.fire(bullet.kind, me, me.panel_x + me.facing, me.panel_y, z, bullet.variants.vulcan, 0, word)
    battle.play_sound(SHOT_SOUND)
    local left = me.timer2 - 1
    me.timer2 = int.u16(left)
    if left <= 0 then
        me.collision:set_region(nil)
        me.anim = 4
        next_action(me, LEAVE)
    end
end

-- 0x080CAA88: it blinks out (4 ticks shown, 4 hidden), then goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = if (me.state :: State).gun == 0 then 1 else 4
        me.timer = LEAVE_TICKS
        me.phase_init = 4
    end
    me.visible = true
    me:hide_from_blind()
    -- (The flag's store again: shown even to a blinded viewer.)
    me.visible = true
    if bit32.btest(me.timer, 2) then
        me.visible = false
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080CA8FC and 0x080CA9C0: the soldier's two phases; then leaving.
local function act(me: Object)
    local a = me.action
    if a == SWORD then
        if me.phase == 0 then
            draw(me)
        elseif me.phase == 4 then
            slash(me)
        else
            error(string.format("ColonelSoul soldier phase %#x reads past its table", me.phase))
        end
    elseif a == GUN then
        if me.phase == 0 then
            aim(me)
        elseif me.phase == 4 then
            fire(me)
        else
            error(string.format("ColonelSoul soldier phase %#x reads past its table", me.phase))
        end
    elseif a == LEAVE then
        leave(me)
    else
        error(string.format("ColonelSoul soldier action %#x reads past its table", a))
    end
end

-- 0x080CA896.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me.visible = true
    me:hide_from_blind()
    if not s.hit and me:take_damage(0) < 0 then
        s.hit = true
        next_action(me, LEAVE)
        c:set_region(nil)
    end
    if not battle.dimmed() then
        act(me)
    end
    c:present()
end

-- 0x080CA858: its sprite and collision (`sub_8011504`), on its panel on the ground with 1 HP; then its first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = if s.gun == 0 then 0 else 3
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        -- (The game goes on into the freed slot and its first tick, which reads a collision it hasn't: nothing
        -- frees a slot of the pool it lacked here while a soldier spawns.)
        me:free()
        return
    end
    me:setup_collision(collision.own_body, collision.hit_by_other_side, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.hp, me.max_hp = HP, HP
    me:set_lifecycle("update")
    me.action = if s.gun == 0 then SWORD else GUN
    tick(me)
end

soldier.kind = define.kind {
    id = "bn5:colonel-soldier",
    pool = "attack",
    state = { gun = "u8", hit = "bool" },
    -- 0x080CA834.
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
            -- 0x080138F2.
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

return soldier
